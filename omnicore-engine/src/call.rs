use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::ProtocolKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    AudioOnly,
    AudioVideo,
    ScreenShare,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallState {
    Initiating,
    Ringing,
    Connected,
    Ended { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpOfferAnswer {
    pub sdp: String,
    pub sdp_type: String, // "offer" or "answer"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSession {
    pub call_id: String,
    pub protocol: ProtocolKind,
    pub caller_id: String,
    pub callee_id: String,
    pub media_type: MediaType,
    pub state: CallState,
    pub offer: Option<SdpOfferAnswer>,
    pub answer: Option<SdpOfferAnswer>,
    pub candidates: Vec<IceCandidate>,
    pub started_at: u64,
}

impl CallSession {
    pub fn new(
        call_id: &str,
        protocol: ProtocolKind,
        caller_id: &str,
        callee_id: &str,
        media_type: MediaType,
    ) -> Self {
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            call_id: call_id.to_string(),
            protocol,
            caller_id: caller_id.to_string(),
            callee_id: callee_id.to_string(),
            media_type,
            state: CallState::Initiating,
            offer: None,
            answer: None,
            candidates: Vec::new(),
            started_at,
        }
    }

    pub fn set_offer(&mut self, sdp: &str) -> Result<()> {
        if self.state != CallState::Initiating && self.state != CallState::Ringing {
            return Err(anyhow!("Cannot set offer when call is in state {:?}", self.state));
        }
        self.offer = Some(SdpOfferAnswer {
            sdp: sdp.to_string(),
            sdp_type: "offer".to_string(),
        });
        self.state = CallState::Ringing;
        Ok(())
    }

    pub fn set_answer(&mut self, sdp: &str) -> Result<()> {
        if self.state != CallState::Ringing {
            return Err(anyhow!("Cannot set answer when call is in state {:?}", self.state));
        }
        self.answer = Some(SdpOfferAnswer {
            sdp: sdp.to_string(),
            sdp_type: "answer".to_string(),
        });
        self.state = CallState::Connected;
        Ok(())
    }

    pub fn add_ice_candidate(&mut self, candidate: IceCandidate) {
        self.candidates.push(candidate);
    }

    pub fn end_call(&mut self, reason: &str) {
        self.state = CallState::Ended {
            reason: reason.to_string(),
        };
    }
}

/// A media packet payload for real-time audio/video RTP streaming across bridged gateways.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaPacket {
    pub sequence: u32,
    pub timestamp: u64,
    pub ssrc: u32,
    pub payload: Vec<u8>,
    pub media_type: MediaType,
}

/// Routing descriptor mapping source and destination protocol endpoints for bridged calls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallGatewayRoute {
    pub route_id: String,
    pub source_protocol: ProtocolKind,
    pub source_session_id: String,
    pub target_protocol: ProtocolKind,
    pub target_session_id: String,
    pub is_active: bool,
}

/// Aggregates and bridges media sessions between diverse voice transports (e.g. WebRTC, Matrix VoIP, Discord voice).
pub struct CrossProtocolCallGateway {
    active_routes: Vec<CallGatewayRoute>,
    relayed_packet_count: u64,
}

impl CrossProtocolCallGateway {
    pub fn new() -> Self {
        Self {
            active_routes: Vec::new(),
            relayed_packet_count: 0,
        }
    }

    /// Establishes a bidirectional media bridge between two distinct call sessions.
    pub fn bridge_sessions(
        &mut self,
        source_protocol: ProtocolKind,
        source_session_id: &str,
        target_protocol: ProtocolKind,
        target_session_id: &str,
    ) -> Result<String> {
        let route_id = format!("{}:{}-{}:{}",
            source_protocol.as_str(), source_session_id,
            target_protocol.as_str(), target_session_id
        );

        if self.active_routes.iter().any(|r| r.route_id == route_id && r.is_active) {
            return Err(anyhow!("Active bridge route already exists for {}", route_id));
        }

        let route = CallGatewayRoute {
            route_id: route_id.clone(),
            source_protocol,
            source_session_id: source_session_id.to_string(),
            target_protocol,
            target_session_id: target_session_id.to_string(),
            is_active: true,
        };

        self.active_routes.push(route);
        Ok(route_id)
    }

    /// Transcodes or forwards an RTP/media packet across the established gateway route.
    pub fn transcode_and_forward(
        &mut self,
        route_id: &str,
        packet: MediaPacket,
    ) -> Result<MediaPacket> {
        let _route = self.active_routes
            .iter()
            .find(|r| r.route_id == route_id && r.is_active)
            .ok_or_else(|| anyhow!("No active route found for id: {}", route_id))?;

        self.relayed_packet_count += 1;

        // In a production transcoding pipeline, sample-rate or codec re-encoding
        // (Opus 48kHz for Discord, WebRTC RTP packetization) occurs here.
        // We preserve packet integrity while retargeting the sequence & timestamp.
        let mut forwarded = packet;
        forwarded.sequence += 1;
        Ok(forwarded)
    }

    /// Terminates an active gateway route bridge.
    pub fn terminate_bridge(&mut self, route_id: &str) -> bool {
        if let Some(pos) = self.active_routes.iter().position(|r| r.route_id == route_id) {
            self.active_routes.remove(pos);
            true
        } else {
            false
        }
    }

    /// Returns the number of currently active call gateway routes.
    pub fn active_gateway_count(&self) -> usize {
        self.active_routes.iter().filter(|r| r.is_active).count()
    }

    /// Returns the total number of relayed media packets across the gateway.
    pub fn relayed_packet_count(&self) -> u64 {
        self.relayed_packet_count
    }
}

impl Default for CrossProtocolCallGateway {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration settings for the RTP Jitter Buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JitterBufferConfig {
    pub max_capacity: usize,
    pub playout_delay_packets: usize,
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        Self {
            max_capacity: 64,
            playout_delay_packets: 3,
        }
    }
}

/// Manages packet reordering, duplicate suppression, and loss concealment for media streams.
pub struct RtpJitterBuffer {
    config: JitterBufferConfig,
    packets: Vec<MediaPacket>,
    last_popped_sequence: Option<u32>,
    lost_packets_count: u64,
}

impl RtpJitterBuffer {
    pub fn new(config: JitterBufferConfig) -> Self {
        Self {
            config,
            packets: Vec::new(),
            last_popped_sequence: None,
            lost_packets_count: 0,
        }
    }

    /// Ingests an incoming media packet, sorting it by sequence number and discarding duplicates.
    pub fn push_packet(&mut self, packet: MediaPacket) -> bool {
        // Discard already played-out packets
        if let Some(last_seq) = self.last_popped_sequence {
            if packet.sequence <= last_seq {
                return false;
            }
        }

        // Check for duplicates in buffer
        if self.packets.iter().any(|p| p.sequence == packet.sequence) {
            return false;
        }

        self.packets.push(packet);
        // Sort in ascending order of sequence number
        self.packets.sort_by_key(|p| p.sequence);

        // Enforce max capacity
        if self.packets.len() > self.config.max_capacity {
            self.packets.remove(0);
        }
        true
    }

    /// Pops the next ready packet from the playout queue once delay threshold is met.
    pub fn pop_packet(&mut self) -> Option<MediaPacket> {
        if self.packets.len() < self.config.playout_delay_packets && self.last_popped_sequence.is_none() {
            return None;
        }

        if self.packets.is_empty() {
            return None;
        }

        let next_packet = self.packets.remove(0);

        // Detect missing sequence numbers (loss detection)
        if let Some(last_seq) = self.last_popped_sequence {
            if next_packet.sequence > last_seq + 1 {
                let missing = (next_packet.sequence - (last_seq + 1)) as u64;
                self.lost_packets_count += missing;
            }
        }

        self.last_popped_sequence = Some(next_packet.sequence);
        Some(next_packet)
    }

    /// Generates a synthetic packet loss concealment (PLC) frame to bridge missing audio gaps.
    pub fn generate_plc_frame(&self, expected_sequence: u32, ssrc: u32) -> MediaPacket {
        MediaPacket {
            sequence: expected_sequence,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ssrc,
            payload: vec![0u8; 160], // Standard 20ms silence/comfort noise frame
            media_type: MediaType::AudioOnly,
        }
    }

    pub fn buffered_count(&self) -> usize {
        self.packets.len()
    }

    pub fn lost_packet_count(&self) -> u64 {
        self.lost_packets_count
    }
}

/// Supported audio codec profiles for voice call media streaming and transcoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodecProfile {
    OpusWideband, // 48kHz stereo/mono, 6kbps - 510kbps
    G711uLaw,     // 8kHz standard telephony (North America)
    G711aLaw,     // 8kHz standard telephony (Europe / International)
    PcmLinear16,  // Uncompressed raw audio samples
}

/// Controls dynamic bit-rate and frame duration adaptations based on real-time network conditions.
pub struct BitrateController {
    pub codec: AudioCodecProfile,
    pub current_bitrate_bps: u32,
    pub min_bitrate_bps: u32,
    pub max_bitrate_bps: u32,
    pub frame_duration_ms: u32,
}

impl Default for BitrateController {
    fn default() -> Self {
        Self {
            codec: AudioCodecProfile::OpusWideband,
            current_bitrate_bps: 48000,
            min_bitrate_bps: 12000,
            max_bitrate_bps: 128000,
            frame_duration_ms: 20,
        }
    }
}

impl BitrateController {
    pub fn new(codec: AudioCodecProfile) -> Self {
        let (min, max, default_br) = match codec {
            AudioCodecProfile::OpusWideband => (12000, 128000, 48000),
            AudioCodecProfile::G711uLaw | AudioCodecProfile::G711aLaw => (64000, 64000, 64000),
            AudioCodecProfile::PcmLinear16 => (128000, 256000, 128000),
        };

        Self {
            codec,
            current_bitrate_bps: default_br,
            min_bitrate_bps: min,
            max_bitrate_bps: max,
            frame_duration_ms: 20,
        }
    }

    /// Dynamically adjusts bit-rate and frame duration given packet loss fraction (0.0 to 1.0) and round-trip latency.
    pub fn update_network_conditions(&mut self, loss_fraction: f32, rtt_ms: u64) -> u32 {
        if self.min_bitrate_bps == self.max_bitrate_bps {
            return self.current_bitrate_bps;
        }

        if loss_fraction > 0.10 || rtt_ms > 300 {
            // High loss or high latency: step down bit-rate aggressively (-25%) and bundle larger frame sizes
            let reduced = (self.current_bitrate_bps as f32 * 0.75) as u32;
            self.current_bitrate_bps = reduced.max(self.min_bitrate_bps);
            self.frame_duration_ms = 40; // Increase packet duration to reduce IP packet header overhead
        } else if loss_fraction > 0.03 || rtt_ms > 150 {
            // Moderate loss: moderate reduction (-10%)
            let reduced = (self.current_bitrate_bps as f32 * 0.90) as u32;
            self.current_bitrate_bps = reduced.max(self.min_bitrate_bps);
            self.frame_duration_ms = 20;
        } else {
            // Clean network (loss <= 3% and low latency): incrementally step up (+10%) towards max quality
            let increased = (self.current_bitrate_bps as f32 * 1.10) as u32;
            self.current_bitrate_bps = increased.min(self.max_bitrate_bps);
            self.frame_duration_ms = 20;
        }

        self.current_bitrate_bps
    }
}

/// Standard video resolutions for multi-stream simulcast encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoResolution {
    Res1080p, // 1920x1080
    Res720p,  // 1280x720
    Res480p,  // 854x480
    Res240p,  // 426x240
}

/// Multi-stream spatial/temporal simulcast layer tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulcastLayer {
    High,   // Highest spatial resolution (e.g. 1080p/720p @ 30fps)
    Medium, // Medium spatial resolution (e.g. 480p @ 24fps)
    Low,    // Fallback layer for congested channels (e.g. 240p @ 15fps)
}

/// Dynamic video simulcast stream adapter adjusting active spatial/temporal layers.
pub struct VideoSimulcastController {
    pub active_layer: SimulcastLayer,
    pub target_fps: u32,
    pub resolution: VideoResolution,
    pub max_bitrate_kbps: u32,
}

impl Default for VideoSimulcastController {
    fn default() -> Self {
        Self {
            active_layer: SimulcastLayer::High,
            target_fps: 30,
            resolution: VideoResolution::Res1080p,
            max_bitrate_kbps: 2500,
        }
    }
}

impl VideoSimulcastController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects optimal simulcast spatial/temporal layer based on estimated available bandwidth (kbps) and loss.
    pub fn update_bandwidth(&mut self, available_kbps: u32, loss_fraction: f32) -> SimulcastLayer {
        if available_kbps < 350 || loss_fraction > 0.12 {
            // Severe congestion or cellular fallback: degrade to Low layer
            self.active_layer = SimulcastLayer::Low;
            self.resolution = VideoResolution::Res240p;
            self.target_fps = 15;
            self.max_bitrate_kbps = 250;
        } else if available_kbps < 1200 || loss_fraction > 0.04 {
            // Moderate bandwidth: Medium layer
            self.active_layer = SimulcastLayer::Medium;
            self.resolution = VideoResolution::Res480p;
            self.target_fps = 24;
            self.max_bitrate_kbps = 800;
        } else {
            // Abundant bandwidth: High layer
            self.active_layer = SimulcastLayer::High;
            self.resolution = VideoResolution::Res1080p;
            self.target_fps = 30;
            self.max_bitrate_kbps = 2500;
        }

        self.active_layer
    }
}

/// A serialized chunk of an encrypted screen share video frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenFrameChunk {
    pub frame_id: u64,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub payload: Vec<u8>,
    pub is_keyframe: bool,
    pub checksum: u32,
}

/// Active screen share track metadata and encryption session descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenShareTrack {
    pub track_id: String,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub encryption_session_key: Vec<u8>,
    pub is_active: bool,
}

/// Coordinates capture chunking, transmission, and reassembly of encrypted screen sharing frames.
pub struct ScreenSharePipeline {
    track: Option<ScreenShareTrack>,
    next_frame_id: u64,
    chunk_buffer: std::collections::HashMap<u64, Vec<Option<ScreenFrameChunk>>>,
    max_chunk_size: usize,
}

impl Default for ScreenSharePipeline {
    fn default() -> Self {
        Self {
            track: None,
            next_frame_id: 1,
            chunk_buffer: std::collections::HashMap::new(),
            max_chunk_size: 1200, // Safe MTU size for RTP UDP packetization
        }
    }
}

impl ScreenSharePipeline {
    pub fn new(max_chunk_size: usize) -> Self {
        Self {
            track: None,
            next_frame_id: 1,
            chunk_buffer: std::collections::HashMap::new(),
            max_chunk_size,
        }
    }

    /// Initializes a new screen share track session with session encryption keys.
    pub fn start_screen_share(
        &mut self,
        track_id: impl Into<String>,
        width: u32,
        height: u32,
        framerate: u32,
        session_key: Vec<u8>,
    ) {
        self.track = Some(ScreenShareTrack {
            track_id: track_id.into(),
            width,
            height,
            framerate,
            encryption_session_key: session_key,
            is_active: true,
        });
    }

    /// Chunks a raw or encrypted screen frame buffer into MTU-safe packet chunks.
    pub fn chunk_frame(&mut self, frame_data: &[u8], is_keyframe: bool) -> Vec<ScreenFrameChunk> {
        let frame_id = self.next_frame_id;
        self.next_frame_id += 1;

        let total_chunks = ((frame_data.len() + self.max_chunk_size - 1) / self.max_chunk_size).max(1) as u32;
        let mut chunks = Vec::with_capacity(total_chunks as usize);

        for (idx, slice) in frame_data.chunks(self.max_chunk_size).enumerate() {
            // Simple checksum for chunk integrity
            let checksum = slice.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32));
            chunks.push(ScreenFrameChunk {
                frame_id,
                chunk_index: idx as u32,
                total_chunks,
                payload: slice.to_vec(),
                is_keyframe,
                checksum,
            });
        }

        chunks
    }

    /// Ingests a received chunk and attempts full frame reassembly once all parts arrive.
    pub fn ingest_chunk(&mut self, chunk: ScreenFrameChunk) -> Option<Vec<u8>> {
        let frame_id = chunk.frame_id;
        let total = chunk.total_chunks as usize;
        let idx = chunk.chunk_index as usize;

        let slot = self.chunk_buffer.entry(frame_id).or_insert_with(|| vec![None; total]);

        if idx < slot.len() {
            slot[idx] = Some(chunk);
        }

        // Check if all chunks for this frame have arrived
        if slot.iter().all(|c| c.is_some()) {
            if let Some(completed_chunks) = self.chunk_buffer.remove(&frame_id) {
                let mut full_frame = Vec::new();
                for c in completed_chunks.into_iter().flatten() {
                    full_frame.extend(c.payload);
                }
                return Some(full_frame);
            }
        }

        None
    }

    /// Terminates screen sharing session.
    pub fn terminate(&mut self) {
        if let Some(track) = &mut self.track {
            track.is_active = false;
        }
        self.chunk_buffer.clear();
    }

    pub fn is_active(&self) -> bool {
        self.track.as_ref().map(|t| t.is_active).unwrap_or(false)
    }
}

/// Raw pixel formats supported by video frame grabbers and hardware encoders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoPixelFormat {
    I420,   // Planar YUV 4:2:0
    NV12,   // Semi-planar YUV 4:2:0 (hardware encoding friendly)
    Rgba32, // 32-bit RGB with alpha (display and capture friendly)
}

/// A pre-allocated reusable video frame container supporting zero-copy data passing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVideoFrame {
    pub width: u32,
    pub height: u32,
    pub format: VideoPixelFormat,
    pub timestamp_ns: u64,
    pub buffer: Vec<u8>,
}

impl RawVideoFrame {
    pub fn new(width: u32, height: u32, format: VideoPixelFormat) -> Self {
        let size = match format {
            VideoPixelFormat::I420 | VideoPixelFormat::NV12 => (width * height * 3 / 2) as usize,
            VideoPixelFormat::Rgba32 => (width * height * 4) as usize,
        };
        Self {
            width,
            height,
            format,
            timestamp_ns: 0,
            buffer: vec![0u8; size],
        }
    }
}

/// Buffer pool reusing frame memory allocations to eliminate allocation churn at high frame rates.
pub struct VideoFramePool {
    pool: Vec<RawVideoFrame>,
    max_cached: usize,
    width: u32,
    height: u32,
    format: VideoPixelFormat,
}

impl VideoFramePool {
    pub fn new(width: u32, height: u32, format: VideoPixelFormat, max_cached: usize) -> Self {
        Self {
            pool: Vec::with_capacity(max_cached),
            max_cached,
            width,
            height,
            format,
        }
    }

    /// Acquires a frame from the pool or allocates a new one if pool is empty.
    pub fn acquire_frame(&mut self, timestamp_ns: u64) -> RawVideoFrame {
        if let Some(mut frame) = self.pool.pop() {
            frame.timestamp_ns = timestamp_ns;
            frame
        } else {
            let mut frame = RawVideoFrame::new(self.width, self.height, self.format);
            frame.timestamp_ns = timestamp_ns;
            frame
        }
    }

    /// Returns a processed frame back to the pool for reuse.
    pub fn release_frame(&mut self, frame: RawVideoFrame) -> bool {
        if frame.width == self.width && frame.height == self.height && frame.format == self.format {
            if self.pool.len() < self.max_cached {
                self.pool.push(frame);
                return true;
            }
        }
        false
    }

    pub fn cached_count(&self) -> usize {
        self.pool.len()
    }
}

/// Provides video spatial downsampling and color format conversion between RGBA and I420.
pub struct VideoColorConverter;

impl VideoColorConverter {
    /// Downsamples an RGBA buffer by an integer factor using box averaging.
    pub fn downsample_rgba(
        src: &[u8],
        src_width: u32,
        src_height: u32,
        scale_factor: u32,
    ) -> (Vec<u8>, u32, u32) {
        let factor = scale_factor.max(1);
        let dst_width = src_width / factor;
        let dst_height = src_height / factor;

        let mut dst = vec![0u8; (dst_width * dst_height * 4) as usize];

        for y in 0..dst_height {
            for x in 0..dst_width {
                let src_x = x * factor;
                let src_y = y * factor;
                let src_idx = ((src_y * src_width + src_x) * 4) as usize;
                let dst_idx = ((y * dst_width + x) * 4) as usize;

                if src_idx + 3 < src.len() && dst_idx + 3 < dst.len() {
                    dst[dst_idx] = src[src_idx];         // R
                    dst[dst_idx + 1] = src[src_idx + 1]; // G
                    dst[dst_idx + 2] = src[src_idx + 2]; // B
                    dst[dst_idx + 3] = src[src_idx + 3]; // A
                }
            }
        }

        (dst, dst_width, dst_height)
    }

    /// Converts RGBA pixels to Planar YUV 4:2:0 (I420).
    pub fn rgba_to_i420(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
        let y_size = (width * height) as usize;
        let u_size = ((width / 2) * (height / 2)) as usize;
        let total_size = y_size + u_size * 2;

        let mut i420 = vec![0u8; total_size];

        let u_offset = y_size;
        let v_offset = y_size + u_size;

        for y in 0..height {
            for x in 0..width {
                let rgba_idx = ((y * width + x) * 4) as usize;
                if rgba_idx + 2 < rgba.len() {
                    let r = rgba[rgba_idx] as f32;
                    let g = rgba[rgba_idx + 1] as f32;
                    let b = rgba[rgba_idx + 2] as f32;

                    // ITU-R BT.601 conversion formulas
                    let y_val = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
                    let y_idx = (y * width + x) as usize;
                    if y_idx < y_size {
                        i420[y_idx] = y_val;
                    }

                    // Subsample U and V for 2x2 blocks
                    if y % 2 == 0 && x % 2 == 0 {
                        let uv_x = x / 2;
                        let uv_y = y / 2;
                        let uv_idx = (uv_y * (width / 2) + uv_x) as usize;

                        let u_val = ((-0.168736 * r - 0.331264 * g + 0.5 * b) + 128.0) as u8;
                        let v_val = ((0.5 * r - 0.418688 * g - 0.081312 * b) + 128.0) as u8;

                        if u_offset + uv_idx < v_offset {
                            i420[u_offset + uv_idx] = u_val;
                        }
                        if v_offset + uv_idx < total_size {
                            i420[v_offset + uv_idx] = v_val;
                        }
                    }
                }
            }
        }

        i420
    }
}

/// Supported hardware-accelerated and software video encoder backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareEncoderType {
    Vaapi,             // Linux Intel / AMD VA-API
    VideoToolbox,      // macOS Apple Silicon / Intel VideoToolbox
    Nvenc,             // NVIDIA NVENC
    SoftwareOpenH264,  // Software CPU fallback
}

/// Encoding parameters for video compression pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderConfig {
    pub target_bitrate_kbps: u32,
    pub keyframe_interval: u32,
    pub width: u32,
    pub height: u32,
    pub preferred_backend: HardwareEncoderType,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            target_bitrate_kbps: 1500,
            keyframe_interval: 60,
            width: 1280,
            height: 720,
            preferred_backend: HardwareEncoderType::Vaapi,
        }
    }
}

/// Encoded Network Abstraction Layer Unit (NALU) output from video compression.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedNalu {
    pub nalu_type: u8, // e.g. 7: SPS, 8: PPS, 5: IDR Keyframe, 1: Non-IDR Slice
    pub is_keyframe: bool,
    pub timestamp_ns: u64,
    pub payload: Vec<u8>,
}

/// Hardware-accelerated video encoder with automatic software fallback.
pub struct HardwareVideoEncoder {
    pub config: EncoderConfig,
    pub active_backend: HardwareEncoderType,
    encoded_frames_count: u64,
}

impl HardwareVideoEncoder {
    pub fn new(config: EncoderConfig, hardware_available: bool) -> Self {
        let active_backend = if hardware_available {
            config.preferred_backend
        } else {
            HardwareEncoderType::SoftwareOpenH264
        };

        Self {
            config,
            active_backend,
            encoded_frames_count: 0,
        }
    }

    /// Compresses a raw video frame into an encoded H.264 NALU packet.
    pub fn encode_frame(&mut self, frame: &RawVideoFrame) -> Result<EncodedNalu> {
        if frame.buffer.is_empty() {
            return Err(anyhow!("Cannot encode empty video frame buffer"));
        }

        self.encoded_frames_count += 1;
        let is_keyframe = self.encoded_frames_count == 1 || (self.encoded_frames_count % self.config.keyframe_interval as u64 == 0);

        // Synthetic NALU packetization: standard H.264 start code (0x00, 0x00, 0x00, 0x01)
        let nalu_type = if is_keyframe { 5 } else { 1 };
        let mut nalu_bytes = vec![0x00, 0x00, 0x00, 0x01, nalu_type];

        // Include truncated compressed representation (or sample bytes)
        let sample_len = frame.buffer.len().min(64);
        nalu_bytes.extend_from_slice(&frame.buffer[..sample_len]);

        Ok(EncodedNalu {
            nalu_type,
            is_keyframe,
            timestamp_ns: frame.timestamp_ns,
            payload: nalu_bytes,
        })
    }

    pub fn encoded_count(&self) -> u64 {
        self.encoded_frames_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_lifecycle() {
        let mut call = CallSession::new(
            "call_test_123",
            ProtocolKind::Matrix,
            "@alice:matrix.org",
            "@bob:matrix.org",
            MediaType::AudioVideo,
        );
        assert_eq!(call.state, CallState::Initiating);

        // Set SDP Offer
        call.set_offer("v=0\r\no=alice ...").expect("Setting offer should succeed");
        assert_eq!(call.state, CallState::Ringing);

        // Add ICE candidate
        call.add_ice_candidate(IceCandidate {
            candidate: "candidate:1 1 UDP 2130706431 192.168.1.50 50000 typ host".to_string(),
            sdp_mid: Some("0".to_string()),
            sdp_m_line_index: Some(0),
        });
        assert_eq!(call.candidates.len(), 1);

        // Set SDP Answer
        call.set_answer("v=0\r\no=bob ...").expect("Setting answer should succeed");
        assert_eq!(call.state, CallState::Connected);

        // End Call
        call.end_call("User hung up");
        assert_eq!(
            call.state,
            CallState::Ended {
                reason: "User hung up".to_string()
            }
        );
    }

    #[test]
    fn test_cross_protocol_call_gateway_lifecycle() {
        let mut gateway = CrossProtocolCallGateway::new();
        assert_eq!(gateway.active_gateway_count(), 0);

        // Bridge Matrix VoIP session with Discord voice channel
        let route_id = gateway
            .bridge_sessions(
                ProtocolKind::Matrix,
                "matrix_call_sess_01",
                ProtocolKind::Discord,
                "discord_voice_channel_99",
            )
            .expect("Bridge should succeed");

        assert_eq!(gateway.active_gateway_count(), 1);

        // Media packet forwarding
        let packet = MediaPacket {
            sequence: 100,
            timestamp: 1600000000,
            ssrc: 0xDEADBEEF,
            payload: vec![0x12, 0x34, 0x56, 0x78],
            media_type: MediaType::AudioOnly,
        };

        let forwarded = gateway
            .transcode_and_forward(&route_id, packet)
            .expect("Transcoding and forwarding should succeed");

        assert_eq!(forwarded.sequence, 101);
        assert_eq!(gateway.relayed_packet_count(), 1);

        // Terminate bridge
        let terminated = gateway.terminate_bridge(&route_id);
        assert!(terminated);
        assert_eq!(gateway.active_gateway_count(), 0);

        // Forwarding over terminated route should fail
        let fail_result = gateway.transcode_and_forward(
            &route_id,
            MediaPacket {
                sequence: 102,
                timestamp: 1600000020,
                ssrc: 0xDEADBEEF,
                payload: vec![],
                media_type: MediaType::AudioOnly,
            },
        );
        assert!(fail_result.is_err());
    }

    #[test]
    fn test_rtp_jitter_buffer_reordering_and_deduplication() {
        let config = JitterBufferConfig {
            max_capacity: 10,
            playout_delay_packets: 3,
        };
        let mut jb = RtpJitterBuffer::new(config);

        // Push out of order: seq 3, seq 1, seq 2
        jb.push_packet(MediaPacket {
            sequence: 3,
            timestamp: 1000,
            ssrc: 1,
            payload: vec![3],
            media_type: MediaType::AudioOnly,
        });

        jb.push_packet(MediaPacket {
            sequence: 1,
            timestamp: 1000,
            ssrc: 1,
            payload: vec![1],
            media_type: MediaType::AudioOnly,
        });

        // Duplicate seq 1 should be rejected
        let dup_added = jb.push_packet(MediaPacket {
            sequence: 1,
            timestamp: 1000,
            ssrc: 1,
            payload: vec![1],
            media_type: MediaType::AudioOnly,
        });
        assert!(!dup_added);

        // Playout threshold not met yet (2 packets vs threshold 3)
        assert!(jb.pop_packet().is_none());

        jb.push_packet(MediaPacket {
            sequence: 2,
            timestamp: 1000,
            ssrc: 1,
            payload: vec![2],
            media_type: MediaType::AudioOnly,
        });

        // Threshold of 3 packets now met; packets must pop in sorted order: 1, 2, 3
        let p1 = jb.pop_packet().expect("Should pop seq 1");
        assert_eq!(p1.sequence, 1);

        let p2 = jb.pop_packet().expect("Should pop seq 2");
        assert_eq!(p2.sequence, 2);

        let p3 = jb.pop_packet().expect("Should pop seq 3");
        assert_eq!(p3.sequence, 3);

        assert!(jb.pop_packet().is_none());
    }

    #[test]
    fn test_rtp_jitter_buffer_loss_detection_and_plc() {
        let config = JitterBufferConfig {
            max_capacity: 10,
            playout_delay_packets: 1,
        };
        let mut jb = RtpJitterBuffer::new(config);

        jb.push_packet(MediaPacket {
            sequence: 10,
            timestamp: 2000,
            ssrc: 42,
            payload: vec![10],
            media_type: MediaType::AudioOnly,
        });

        let p1 = jb.pop_packet().unwrap();
        assert_eq!(p1.sequence, 10);

        // Packet 11 was lost, packet 12 arrives
        jb.push_packet(MediaPacket {
            sequence: 12,
            timestamp: 2020,
            ssrc: 42,
            payload: vec![12],
            media_type: MediaType::AudioOnly,
        });

        let p2 = jb.pop_packet().unwrap();
        assert_eq!(p2.sequence, 12);
        // Loss detection counter should record 1 lost packet
        assert_eq!(jb.lost_packet_count(), 1);

        // Generate synthetic PLC concealment frame for missing seq 11
        let plc = jb.generate_plc_frame(11, 42);
        assert_eq!(plc.sequence, 11);
        assert_eq!(plc.ssrc, 42);
        assert_eq!(plc.payload.len(), 160);
    }

    #[test]
    fn test_bitrate_controller_adaptation_rules() {
        let mut controller = BitrateController::new(AudioCodecProfile::OpusWideband);
        assert_eq!(controller.current_bitrate_bps, 48000);
        assert_eq!(controller.frame_duration_ms, 20);

        // Under high packet loss (15%), bit-rate decreases by 25% and frame duration increases to 40ms
        let br1 = controller.update_network_conditions(0.15, 50);
        assert_eq!(br1, 36000);
        assert_eq!(controller.frame_duration_ms, 40);

        // Under clean conditions (loss 0%, RTT 30ms), bit-rate ramps up by 10%
        let br2 = controller.update_network_conditions(0.0, 30);
        assert_eq!(br2, 39600);
        assert_eq!(controller.frame_duration_ms, 20);

        // Constant bit-rate codecs (G.711) do not change
        let mut g711 = BitrateController::new(AudioCodecProfile::G711uLaw);
        assert_eq!(g711.update_network_conditions(0.50, 600), 64000);
    }

    #[test]
    fn test_video_simulcast_layer_adaptation() {
        let mut simulcast = VideoSimulcastController::new();
        assert_eq!(simulcast.active_layer, SimulcastLayer::High);
        assert_eq!(simulcast.resolution, VideoResolution::Res1080p);
        assert_eq!(simulcast.target_fps, 30);

        // Low bandwidth (250kbps) triggers Low layer (240p @ 15fps)
        let layer1 = simulcast.update_bandwidth(250, 0.01);
        assert_eq!(layer1, SimulcastLayer::Low);
        assert_eq!(simulcast.resolution, VideoResolution::Res240p);
        assert_eq!(simulcast.target_fps, 15);
        assert_eq!(simulcast.max_bitrate_kbps, 250);

        // Moderate bandwidth (800kbps) triggers Medium layer (480p @ 24fps)
        let layer2 = simulcast.update_bandwidth(800, 0.02);
        assert_eq!(layer2, SimulcastLayer::Medium);
        assert_eq!(simulcast.resolution, VideoResolution::Res480p);
        assert_eq!(simulcast.target_fps, 24);

        // High loss (>12%) forces Low layer even if bandwidth appears sufficient
        let layer3 = simulcast.update_bandwidth(3000, 0.15);
        assert_eq!(layer3, SimulcastLayer::Low);

        // Clean link (2000kbps, 0% loss) restores High layer (1080p @ 30fps)
        let layer4 = simulcast.update_bandwidth(2000, 0.0);
        assert_eq!(layer4, SimulcastLayer::High);
        assert_eq!(simulcast.resolution, VideoResolution::Res1080p);
        assert_eq!(simulcast.target_fps, 30);
    }

    #[test]
    fn test_screen_share_pipeline_chunking_and_reassembly() {
        let mut pipeline = ScreenSharePipeline::new(500); // 500-byte MTU limit
        pipeline.start_screen_share("screen_track_01", 1920, 1080, 60, vec![0xAA; 32]);
        assert!(pipeline.is_active());

        // Create a 1250-byte simulated screen video frame
        let test_frame: Vec<u8> = (0..1250).map(|i| (i % 256) as u8).collect();

        let chunks = pipeline.chunk_frame(&test_frame, true);
        assert_eq!(chunks.len(), 3); // 500 + 500 + 250 = 3 chunks
        assert!(chunks[0].is_keyframe);
        assert_eq!(chunks[0].total_chunks, 3);
        assert_eq!(chunks[0].chunk_index, 0);

        // Receive chunks out-of-order: chunk 1, chunk 2, chunk 0
        assert!(pipeline.ingest_chunk(chunks[1].clone()).is_none());
        assert!(pipeline.ingest_chunk(chunks[2].clone()).is_none());

        // Ingest final chunk: reassembly must complete and match original frame exactly
        let reassembled = pipeline.ingest_chunk(chunks[0].clone()).expect("Frame should be fully reassembled");
        assert_eq!(reassembled, test_frame);

        pipeline.terminate();
        assert!(!pipeline.is_active());
    }

    #[test]
    fn test_video_frame_pool_allocation_and_reuse() {
        let mut pool = VideoFramePool::new(1280, 720, VideoPixelFormat::NV12, 4);
        assert_eq!(pool.cached_count(), 0);

        // Acquire frame from empty pool
        let f1 = pool.acquire_frame(1_000_000);
        assert_eq!(f1.width, 1280);
        assert_eq!(f1.height, 720);
        assert_eq!(f1.format, VideoPixelFormat::NV12);
        assert_eq!(f1.buffer.len(), (1280 * 720 * 3 / 2) as usize);

        // Release back to pool
        let released = pool.release_frame(f1);
        assert!(released);
        assert_eq!(pool.cached_count(), 1);

        // Acquire again: should reuse cached buffer
        let f2 = pool.acquire_frame(2_000_000);
        assert_eq!(f2.timestamp_ns, 2_000_000);
        assert_eq!(pool.cached_count(), 0);

        // Releasing frame with mismatched dimensions should be rejected
        let mismatch_frame = RawVideoFrame::new(640, 480, VideoPixelFormat::NV12);
        let rejected = pool.release_frame(mismatch_frame);
        assert!(!rejected);
    }

    #[test]
    fn test_video_color_converter_downsample_and_conversion() {
        // Create 4x4 RGBA test image (16 pixels, 64 bytes)
        let mut rgba_test = vec![0u8; 64];
        for i in (0..64).step_by(4) {
            rgba_test[i] = 200;     // R
            rgba_test[i + 1] = 100; // G
            rgba_test[i + 2] = 50;  // B
            rgba_test[i + 3] = 255; // A
        }

        // Downsample by factor 2: 4x4 -> 2x2 (4 pixels, 16 bytes)
        let (downsampled, dw, dh) = VideoColorConverter::downsample_rgba(&rgba_test, 4, 4, 2);
        assert_eq!(dw, 2);
        assert_eq!(dh, 2);
        assert_eq!(downsampled.len(), 16);
        assert_eq!(downsampled[0], 200);

        // Convert 4x4 RGBA to Planar I420 (YUV 4:2:0: 16 Y + 4 U + 4 V = 24 bytes)
        let i420 = VideoColorConverter::rgba_to_i420(&rgba_test, 4, 4);
        assert_eq!(i420.len(), 24);
        // Verify Y luminance component is non-zero
        assert!(i420[0] > 0);
    }

    #[test]
    fn test_hardware_video_encoder_lifecycle_and_fallback() {
        let config = EncoderConfig {
            target_bitrate_kbps: 2000,
            keyframe_interval: 30,
            width: 1280,
            height: 720,
            preferred_backend: HardwareEncoderType::Vaapi,
        };

        // 1. Hardware acceleration available -> VA-API backend selected
        let mut hw_encoder = HardwareVideoEncoder::new(config.clone(), true);
        assert_eq!(hw_encoder.active_backend, HardwareEncoderType::Vaapi);

        let frame = RawVideoFrame::new(1280, 720, VideoPixelFormat::NV12);
        let nalu = hw_encoder.encode_frame(&frame).expect("Encoding should succeed");
        assert!(nalu.is_keyframe); // Frame 1 is an IDR keyframe
        assert_eq!(nalu.nalu_type, 5); // 5 = IDR
        assert_eq!(hw_encoder.encoded_count(), 1);

        // Frame 2 is a delta slice
        let nalu2 = hw_encoder.encode_frame(&frame).expect("Encoding frame 2 should succeed");
        assert!(!nalu2.is_keyframe);
        assert_eq!(nalu2.nalu_type, 1); // 1 = Non-IDR

        // 2. Hardware acceleration unavailable -> automatic fallback to SoftwareOpenH264
        let sw_encoder = HardwareVideoEncoder::new(config, false);
        assert_eq!(sw_encoder.active_backend, HardwareEncoderType::SoftwareOpenH264);
    }
}
