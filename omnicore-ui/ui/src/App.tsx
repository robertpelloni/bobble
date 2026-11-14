import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

/* OmniCore consolidated dashboard — ONE page, no tabs/subpages.
   Layout (highest value first, left to right, top to bottom):
     1. Conversation (center) — the core job: talk to a person on any network
     2. People (left) — unified identities across Signal/Matrix/Discord/etc.
     3. Bridges + Routing + Activity (right rail) — will it deliver, and did it?
     4. Security, Groups/Mesh/Calls, Engine (bottom band) — trust + full capability map
   Every engine module in omnicore-engine/src is represented below; each
   capability row names its engine symbol (e.g. MultipathRouter::route_hedged_racing)
   so coverage can be audited against the Rust source. */

interface BridgeStatusInfo { name: string; protocol: string; status: string; endpoint: string; latency_ms: number; fallback_priority: number; connected: boolean; }
interface ContactInfo { id: number; name: string; unified_id: string; primary_protocol: string; fallback_protocol: string; avatar_initials: string; status: string; mapped_protocols: string[]; }
interface MessageItem { id: string; sender: string; recipient: string; content: string; protocol: string; timestamp: string; is_outgoing: boolean; status: string; }
interface SystemMetrics { version: string; active_bridges: number; queued_messages: number; encryption: string; uptime_secs: number; server_running: boolean; }
interface CapabilityInfo { area: string; name: string; description: string; engine_symbol: string; status: string; }
interface RoutingProfileInfo { id: string; label: string; description: string; engine_symbol: string; recommended: boolean; }
interface SecurityStatus { identity_name: string; unified_id: string; fingerprint: string; identity_verified: boolean; tor_enabled: boolean; tor_proxy_url: string; tor_status: string; backup_enabled: boolean; backup_description: string; ratchet_description: string; encryption: string; }
interface NetworkStatus { federation_channels: number; federation_description: string; mesh_peers: number; mesh_description: string; discovery_description: string; active_calls: number; call_description: string; outbox_queued: number; outbox_description: string; telemetry_summary: string; appliance_action: string; reputation_description: string; daemon_url: string; daemon_sessions: number; }
interface ActivityEvent { id: string; kind: string; detail: string; timestamp: string; }
interface ChannelInfo { id: string; name: string; topic: string; bridge_protocol: string; member_count: number; event_count: number; }
interface DaemonStatus { url: string; running: boolean; sessions: number; subscribed_channel: string | null; }

const PROTOCOLS = ['Signal', 'Matrix', 'Discord', 'Simplex', 'Tox', 'iMessage', 'SMS', 'Nostr'];

const DEFAULT_METRICS: SystemMetrics = { version: '1.1.0', active_bridges: 4, queued_messages: 0, encryption: 'SQLCipher (AES-256-GCM)', uptime_secs: 120, server_running: true };

const DEFAULT_CONTACTS: ContactInfo[] = [
  { id: 1, name: 'Alice Montgomery', unified_id: 'alice_unified_1', primary_protocol: 'Signal', fallback_protocol: 'Matrix', avatar_initials: 'AM', status: 'Online', mapped_protocols: ['Signal', 'Matrix', 'SMS'] },
  { id: 2, name: 'Bob Sterling', unified_id: 'bob_unified_2', primary_protocol: 'Matrix', fallback_protocol: 'Discord', avatar_initials: 'BS', status: 'Away', mapped_protocols: ['Matrix', 'Discord'] },
  { id: 3, name: 'Charlie Vance', unified_id: 'charlie_unified_3', primary_protocol: 'Discord', fallback_protocol: 'Signal', avatar_initials: 'CV', status: 'Offline', mapped_protocols: ['Discord', 'Signal'] },
  { id: 4, name: 'Dana Scully', unified_id: 'dana_unified_4', primary_protocol: 'Simplex', fallback_protocol: 'Tox', avatar_initials: 'DS', status: 'Online', mapped_protocols: ['Simplex', 'Tox'] },
];

const DEFAULT_BRIDGES: BridgeStatusInfo[] = [
  { name: 'Signal Daemon Bridge', protocol: 'Signal', status: 'Connected', endpoint: 'unix:///var/run/signald/signald.sock', latency_ms: 12, fallback_priority: 1, connected: true },
  { name: 'Matrix Synapse/Dendrite', protocol: 'Matrix', status: 'Connected', endpoint: 'https://matrix.org', latency_ms: 45, fallback_priority: 2, connected: true },
  { name: 'Discord Gateway Bridge', protocol: 'Discord', status: 'Connected', endpoint: 'wss://gateway.discord.gg', latency_ms: 38, fallback_priority: 3, connected: true },
  { name: 'SimpleX Chat Isolated Broker', protocol: 'Simplex', status: 'Standby', endpoint: 'simplex://broker.v3.local', latency_ms: 60, fallback_priority: 4, connected: false },
  { name: 'Tox Distributed P2P Core', protocol: 'Tox', status: 'Standby', endpoint: 'dht://bootstrap.tox.chat', latency_ms: 95, fallback_priority: 5, connected: false },
  { name: 'iMessage BlueBubbles Relay', protocol: 'iMessage', status: 'Configured', endpoint: 'https://relay.imessage.local', latency_ms: 50, fallback_priority: 6, connected: false },
  { name: 'SMS / Cellular Modem', protocol: 'SMS', status: 'Ready', endpoint: 'serial:///dev/ttyUSB0', latency_ms: 110, fallback_priority: 7, connected: true },
  { name: 'Nostr Relay Pool', protocol: 'Nostr', status: 'Standby', endpoint: 'wss://relay.damus.io', latency_ms: 80, fallback_priority: 8, connected: false },
];

const DEFAULT_MESSAGES: MessageItem[] = [
  { id: 'msg_1', sender: 'Alice Montgomery', recipient: 'You', content: 'Hey Robert! Is OmniCore cross-protocol routing active?', protocol: 'Signal', timestamp: '10:42 AM', is_outgoing: false, status: 'delivered' },
  { id: 'msg_2', sender: 'You', recipient: 'Alice Montgomery', content: 'Yes — routing across Signal, Matrix, Simplex, and Discord seamlessly.', protocol: 'Signal (fallback: Matrix)', timestamp: '10:43 AM', is_outgoing: true, status: 'delivered' },
  { id: 'msg_3', sender: 'Bob Sterling', recipient: 'You', content: 'Confirming bridge sync token received via Matrix homeserver.', protocol: 'Matrix', timestamp: '10:45 AM', is_outgoing: false, status: 'delivered' },
];

const DEFAULT_PROFILES: RoutingProfileInfo[] = [
  { id: 'optimal_cascade', label: 'Optimal cascade (recommended)', description: 'Health + latency ordered cascade; bypasses open circuits.', engine_symbol: 'MultipathRouter::route_with_optimal_cascade', recommended: true },
  { id: 'hedged_racing', label: 'Hedged racing', description: 'Race bridges in parallel for lowest tail latency.', engine_symbol: 'MultipathRouter::route_hedged_racing', recommended: false },
  { id: 'adaptive_retry', label: 'Adaptive fallback retry', description: 'Multi-pass retry with half-open recovery.', engine_symbol: 'MultipathRouter::route_with_adaptive_fallback_retry', recommended: false },
  { id: 'confirmed_receipt', label: 'Confirmed receipt', description: 'Wait for signed acks; cascade on timeout.', engine_symbol: 'MultipathRouter::route_with_confirmed_receipt', recommended: false },
  { id: 'exact_once', label: 'Exact-once delivery', description: 'Dedup ledger prevents duplicate sends.', engine_symbol: 'MultipathRouter::route_with_exact_once_deduplication', recommended: false },
  { id: 'canary_guard', label: 'Canary drop detection', description: 'Probe for silent drops before dispatch.', engine_symbol: 'MultipathRouter::route_with_canary_drop_detection', recommended: false },
  { id: 'percentile_bounded', label: 'Percentile bounded (p95)', description: 'Deadline bounded by historical p95 latency.', engine_symbol: 'MultipathRouter::route_with_percentile_bounded_cascade', recommended: false },
  { id: 'bandwidth_throttled', label: 'Bandwidth throttled', description: 'Respect per-transport byte quotas.', engine_symbol: 'MultipathRouter::route_with_bandwidth_throttled_cascade', recommended: false },
  { id: 'cost_qos', label: 'Cost & QoS optimized', description: 'Cheapest / lowest-drain route that meets QoS.', engine_symbol: 'MultipathRouter::route_with_cost_and_qos_optimization', recommended: false },
  { id: 'penalty_weighted', label: 'Penalty weighted', description: 'Avoid degraded bridges before they trip.', engine_symbol: 'MultipathRouter::route_with_penalty_weighted_cascade', recommended: false },
];

const DEFAULT_CAPABILITIES: CapabilityInfo[] = [
  ...['Optimal cascade|Orders bridges by health/latency, bypasses open circuits.|route_with_optimal_cascade', 'Hedged racing|Races primary + fallback concurrently.|route_hedged_racing', 'Adaptive fallback retry|Multi-pass delivery with half-open recovery.|route_with_adaptive_fallback_retry', 'Flapping resilience|Exponential cooldown for flapping bridges.|route_with_flapping_resilience', 'Penalty-weighted cascade|Latency + failure penalty scoring.|route_with_penalty_weighted_cascade', 'Unified contact routing|Per-protocol address translation per hop.|route_unified_message', 'Fragmentation / MTU|Chunks payloads for SMS/Tox transports.|route_with_fragmentation_adaptive_cascade', 'Content transcoding|Rich text to plain/GSM-7 for SMS.|route_with_transcoding_cascade', 'Confirmed receipt|Signed acks; cascade on timeout.|route_with_confirmed_receipt', 'Concurrency bounded|Per-bridge in-flight permits.|route_with_concurrency_bounded_cascade', 'Anomaly damped|EMA-clamps latency spikes.|route_with_anomaly_damped_cascade', 'Exact-once dedup|120s egress digest ledger.|route_with_exact_once_deduplication', 'Canary drop detection|Nonces detect silent drops.|route_with_canary_drop_detection', 'Percentile bounded|p50/p95 latency envelopes.|route_with_percentile_bounded_cascade', 'Dispersal balancing|Rotates egress across bridges.|route_with_dispersal_balancing', 'Bandwidth throttled|Sliding-window byte quotas.|route_with_bandwidth_throttled_cascade', 'Cost & QoS optimizer|Money, battery, latency balance.|route_with_cost_and_qos_optimization', 'Circuit rehabilitation|Decays trips on stable transports.|route_with_rehabilitated_circuits', 'Active health probing|Heartbeat circuit recovery.|route_with_active_health_probing', 'Starvation prevention|Probes never-tried bridges.|route_with_starvation_prevention', 'Integrity + replay guard|Digests + nonce cache.|route_with_replay_and_integrity_guard'].map((s) => { const [name, description, sym] = s.split('|'); return { area: 'Routing', name, description, engine_symbol: `MultipathRouter::${sym}`, status: 'Available in engine' }; }),
  { area: 'Federation', name: 'Group channels + vector clocks', description: 'Causal replication, deterministic conflicts.', engine_symbol: 'FederationManager / VectorClock', status: 'Available in engine' },
  { area: 'Federation', name: 'Delivery ledger + handover', description: 'Per-member receipts; signed channel migration.', engine_symbol: 'FederatedDeliveryLedger / handover_channel_bridge', status: 'Available in engine' },
  { area: 'Calls & Media', name: 'Call gateway + jitter buffer', description: 'WebRTC/Matrix/Discord voice; PLC concealment.', engine_symbol: 'CrossProtocolCallGateway / RtpJitterBuffer', status: 'Available in engine' },
  { area: 'Calls & Media', name: 'Simulcast, screen share, HW codecs', description: 'Adaptive layers, encrypted share, HW encode/decode.', engine_symbol: 'VideoSimulcastController / HardwareVideoEncoder', status: 'Available in engine' },
  { area: 'Mesh & Discovery', name: 'Store-and-forward mesh', description: 'Multi-hop BLE/Wi-Fi Direct relay with TTL.', engine_symbol: 'MeshRouter / MeshTransportDaemon', status: 'Engine ready; native HCI driver pending' },
  { area: 'Security', name: 'Ratchet, identity, backup, Tor', description: 'Forward secrecy, QR identity, ZK backup, SOCKS5.', engine_symbol: 'RatchetSession / IdentityCard / TorManager', status: 'Available in engine' },
  { area: 'System', name: 'Outbox, telemetry, daemon, SQLCipher', description: 'Offline queue, appliance health, WS IPC, encrypted DB.', engine_symbol: 'OutboxManager / TelemetryCollector / WebSocketServer', status: 'Available in engine' },
];

const DEFAULT_SECURITY: SecurityStatus = { identity_name: 'Local User', unified_id: 'local_user', fingerprint: 'E8B2-3F1A-99C0-D44E-71B8-502C-129A-FE88', identity_verified: true, tor_enabled: false, tor_proxy_url: 'socks5h://127.0.0.1:9050', tor_status: 'Disabled', backup_enabled: true, backup_description: 'Zero-knowledge encrypted chunks + manifest (ZeroKnowledgeBackup)', ratchet_description: 'Double Ratchet sessions + pre-key bundles in SQLCipher key_store', encryption: 'SQLCipher (AES-256-GCM)' };
const DEFAULT_NETWORK: NetworkStatus = { federation_channels: 1, federation_description: 'Federated group channels with vector-clock causal sync + delivery ledger', mesh_peers: 0, mesh_description: 'BLE / Wi-Fi Direct store-and-forward mesh. No native HCI driver yet.', discovery_description: 'LAN UDP beacon discovery with cached peers (PeerDiscovery)', active_calls: 0, call_description: 'WebRTC / Matrix VoIP / Discord voice gateway; jitter buffer + simulcast ready', outbox_queued: 0, outbox_description: 'Offline outbox: exponential backoff, max 5 attempts (OutboxManager)', telemetry_summary: 'CPU 42°C · RAM 256 / 4096 MB · Disk 28.4 GB free · Battery 98%', appliance_action: 'Normal', reputation_description: 'Peer trust tiers + spam rate limiting (ReputationEngine)', daemon_url: 'ws://127.0.0.1:18765', daemon_sessions: 0 };
const DEFAULT_ACTIVITY: ActivityEvent[] = [
  { id: 'evt_1', kind: 'DISPATCH_OK', detail: 'msg_1 routed to Alice Montgomery via Signal IPC (12 ms)', timestamp: '10:43 AM' },
  { id: 'evt_2', kind: 'MULTI_ROUTE', detail: 'msg_2 dual-path Signal primary, Matrix fallback armed', timestamp: '10:43 AM' },
  { id: 'evt_3', kind: 'SYNC_TOKEN', detail: 'Matrix homeserver sync token refreshed', timestamp: '10:45 AM' },
];
const DEFAULT_CHANNELS: ChannelInfo[] = [{ id: 'chan_ops', name: 'ops-room', topic: 'Federated operations channel', bridge_protocol: 'Matrix', member_count: 1, event_count: 0 }];

/* Accessible tooltip: a "?" marker that reveals guidance on hover/focus.
   Uses a real <button> so keyboard users can reach the explanation. */
function Tip({ text }: { text: string }) {
  return (
    <span className="relative inline-flex group align-middle">
      <button type="button" aria-label={text} className="w-4 h-4 rounded-full border border-slate-600 text-slate-400 text-[10px] leading-none inline-flex items-center justify-center hover:text-white hover:border-slate-400 focus:outline-none focus:ring-1 focus:ring-sky-500">?</button>
      <span role="tooltip" className="pointer-events-none absolute left-1/2 -translate-x-1/2 bottom-full mb-2 w-56 rounded bg-slate-800 border border-slate-600 px-2.5 py-2 text-[11px] leading-snug text-slate-200 opacity-0 invisible group-hover:opacity-100 group-hover:visible group-focus-within:opacity-100 group-focus-within:visible z-50 shadow-xl">{text}</span>
    </span>
  );
}

function Section({ title, tip, action, children }: { title: string; tip: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <section className="border-b border-slate-800 px-4 py-3">
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-xs font-bold uppercase tracking-wide text-slate-300 flex items-center gap-1.5">{title} <Tip text={tip} /></h2>
        {action}
      </div>
      {children}
    </section>
  );
}

function Pill({ ok, children }: { ok: boolean; children: React.ReactNode }) {
  return <span className={`text-[10px] font-semibold px-1.5 py-0.5 rounded border ${ok ? 'bg-emerald-950 text-emerald-300 border-emerald-800' : 'bg-slate-800 text-slate-400 border-slate-700'}`}>{children}</span>;
}

export default function App() {
  const isTauri = typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI_IPC__' in window);
  const [engineStatus, setEngineStatus] = useState('Initializing…');
  const [metrics, setMetrics] = useState(DEFAULT_METRICS);
  const [contacts, setContacts] = useState(DEFAULT_CONTACTS);
  const [selectedContact, setSelectedContact] = useState(DEFAULT_CONTACTS[0]);
  const [messages, setMessages] = useState(DEFAULT_MESSAGES);
  const [bridges, setBridges] = useState(DEFAULT_BRIDGES);
  const [profiles, setProfiles] = useState(DEFAULT_PROFILES);
  const [activeProfile, setActiveProfile] = useState('optimal_cascade');
  const [capabilities, setCapabilities] = useState(DEFAULT_CAPABILITIES);
  const [security, setSecurity] = useState(DEFAULT_SECURITY);
  const [network, setNetwork] = useState(DEFAULT_NETWORK);
  const [activity, setActivity] = useState(DEFAULT_ACTIVITY);
  const [channels, setChannels] = useState(DEFAULT_CHANNELS);
  const [inputText, setInputText] = useState('');
  const [primaryProtocol, setPrimaryProtocol] = useState('Signal');
  const [fallbackProtocol, setFallbackProtocol] = useState('Matrix');
  const [searchQuery, setSearchQuery] = useState('');
  const [capFilter, setCapFilter] = useState('All');
  const [showAddContact, setShowAddContact] = useState(false);
  const [newContactName, setNewContactName] = useState('');
  const [newContactPrimary, setNewContactPrimary] = useState('Signal');
  const [newContactFallback, setNewContactFallback] = useState('Matrix');
  const [showAddChannel, setShowAddChannel] = useState(false);
  const [newChannelName, setNewChannelName] = useState('');
  const [newChannelTopic, setNewChannelTopic] = useState('');
  const [handoverTargets, setHandoverTargets] = useState<Record<string, string>>({});
  const [groupMemberId, setGroupMemberId] = useState('');
  const [groupPostText, setGroupPostText] = useState('');
  const [groupChannelId, setGroupChannelId] = useState('chan_ops');
  const [daemon, setDaemon] = useState<DaemonStatus>({ url: 'ws://127.0.0.1:18765', running: false, sessions: 0, subscribed_channel: null });
  const [subChannel, setSubChannel] = useState('ops-room');
  const [showTrayMenu, setShowTrayMenu] = useState(false);
  const [notice, setNotice] = useState('');

  const logLocal = (kind: string, detail: string) => setActivity((p) => [{ id: `evt_${Date.now()}`, kind, detail, timestamp: 'Just now' }, ...p].slice(0, 50));

  useEffect(() => {
    if (!isTauri) { setEngineStatus('Preview mode — engine data is illustrative; launch the desktop app for live state'); return; }
    // init_engine opens the SQLCipher DB and restores persisted contacts/
    // history first; only then fetch contacts/messages so restored state
    // wins over the in-memory defaults.
    invoke('init_engine').then((r) => {
      setEngineStatus(r as string);
      invoke('get_contacts').then((res) => { const l = res as ContactInfo[]; if (l.length) { setContacts(l); setSelectedContact(l[0]); } }).catch(console.error);
      invoke('get_messages').then((res) => setMessages(res as MessageItem[])).catch(console.error);
      invoke('get_activity_log').then((res) => setActivity(res as ActivityEvent[])).catch(console.error);
      invoke('get_daemon_status').then((res) => setDaemon(res as DaemonStatus)).catch(console.error);
    }).catch((e) => setEngineStatus(`Engine error: ${e}`));
    invoke('get_system_metrics').then((r) => setMetrics(r as SystemMetrics)).catch(console.error);
    invoke('get_contacts').then((r) => { const l = r as ContactInfo[]; if (l.length) { setContacts(l); setSelectedContact(l[0]); } }).catch(console.error);
    invoke('get_messages').then((r) => setMessages(r as MessageItem[])).catch(console.error);
    invoke('get_bridges').then((r) => setBridges(r as BridgeStatusInfo[])).catch(console.error);
    invoke('get_routing_profiles').then((r) => setProfiles(r as RoutingProfileInfo[])).catch(console.error);
    invoke('get_capabilities').then((r) => setCapabilities(r as CapabilityInfo[])).catch(console.error);
    invoke('get_security_status').then((r) => setSecurity(r as SecurityStatus)).catch(console.error);
    invoke('get_network_status').then((r) => setNetwork(r as NetworkStatus)).catch(console.error);
    invoke('get_activity_log').then((r) => setActivity(r as ActivityEvent[])).catch(console.error);
    invoke('get_channels').then((r) => setChannels(r as ChannelInfo[])).catch(console.error);
    invoke('get_daemon_status').then((r) => setDaemon(r as DaemonStatus)).catch(console.error);
  }, [isTauri]);

  const handleSendMessage = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputText.trim()) return;
    if (isTauri) {
      invoke('send_message', { recipient: selectedContact.name, content: inputText, protocol: primaryProtocol, fallbackProtocol })
        .then((r) => { setMessages((p) => [...p, r as MessageItem]); setInputText(''); invoke('get_activity_log').then((a) => setActivity(a as ActivityEvent[])).catch(console.error); })
        .catch((err) => setNotice(`Send failed: ${err}`));
    } else {
      setMessages((p) => [...p, { id: `msg_${Date.now()}`, sender: 'You', recipient: selectedContact.name, content: inputText, protocol: `${primaryProtocol} (fallback: ${fallbackProtocol})`, timestamp: 'Just now', is_outgoing: true, status: 'dispatched' }]);
      logLocal('DISPATCH_OK', `Message to ${selectedContact.name} via ${primaryProtocol} (fallback ${fallbackProtocol}) · profile ${activeProfile}`);
      setInputText('');
    }
  };

  const handleToggleBridge = (protocol: string) => {
    if (isTauri) {
      invoke('toggle_bridge', { protocol }).then((u) => { const b = u as BridgeStatusInfo; setBridges((p) => p.map((x) => (x.protocol === b.protocol ? b : x))); }).catch(console.error);
    } else {
      setBridges((p) => p.map((b) => (b.protocol === protocol ? { ...b, connected: !b.connected, status: !b.connected ? 'Connected' : 'Disconnected' } : b)));
      logLocal('BRIDGE_TOGGLE', `${protocol} bridge toggled`);
    }
  };

  const handleToggleServer = () => {
    if (isTauri) invoke('toggle_servers').then((r) => setMetrics((p) => ({ ...p, server_running: r as boolean }))).catch(console.error);
    else { setMetrics((p) => ({ ...p, server_running: !p.server_running })); logLocal('SERVERS', metrics.server_running ? 'Routing workers paused' : 'Routing workers resumed'); }
  };

  const handleProfileChange = (id: string) => {
    setActiveProfile(id);
    if (isTauri) invoke('set_routing_profile', { profileId: id }).catch(console.error);
    else logLocal('ROUTING_PROFILE', `Active routing profile set to ${id}`);
  };

  const handleToggleTor = () => {
    if (isTauri) invoke('toggle_tor').then((on) => { setSecurity((p) => ({ ...p, tor_enabled: on as boolean, tor_status: (on as boolean) ? 'CircuitEstablished { circuit_hops: 3 }' : 'Disabled' })); invoke('get_activity_log').then((a) => setActivity(a as ActivityEvent[])).catch(console.error); }).catch(console.error);
    else { setSecurity((p) => ({ ...p, tor_enabled: !p.tor_enabled, tor_status: !p.tor_enabled ? 'CircuitEstablished (3 hops)' : 'Disabled' })); logLocal(security.tor_enabled ? 'TOR_OFF' : 'TOR_ON', `Tor SOCKS5 routing ${security.tor_enabled ? 'disabled' : 'enabled'}`); }
  };

  const handleProbe = () => {
    if (isTauri) invoke('run_bridge_probe').then((a) => setActivity(a as ActivityEvent[])).catch(console.error);
    else logLocal('HEALTH_PROBE', 'Heartbeat probe sent to all open/half-open circuits (BridgeHeartbeatProber)');
    setNotice('Bridge health probe sent — see Activity below.');
  };

  const handleExportIdentity = () => {
    if (isTauri) invoke('export_identity_card').then((j) => { setNotice(`Identity card exported (${(j as string).length} bytes JSON). Fingerprint: ${security.fingerprint}`); }).catch((e) => setNotice(`Export failed: ${e}`));
    else setNotice(`Identity card (preview): ${security.identity_name} · ${security.fingerprint} · verified=${security.identity_verified}`);
  };

  const handleAddContact = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newContactName.trim()) return;
    if (isTauri) {
      invoke('add_contact', { name: newContactName, primaryProtocol: newContactPrimary, fallbackProtocol: newContactFallback, mappedProtocols: [newContactPrimary, newContactFallback] })
        .then((c) => { const n = c as ContactInfo; setContacts((p) => [...p, n]); setSelectedContact(n); setNewContactName(''); setShowAddContact(false); }).catch(console.error);
    } else {
      const n: ContactInfo = { id: Date.now(), name: newContactName, unified_id: `contact_unified_${Date.now()}`, primary_protocol: newContactPrimary, fallback_protocol: newContactFallback, avatar_initials: newContactName.split(' ').map((w) => w[0]).join('').toUpperCase() || 'U', status: 'Online', mapped_protocols: [newContactPrimary, newContactFallback] };
      setContacts((p) => [...p, n]); setSelectedContact(n); setNewContactName(''); setShowAddContact(false);
    }
  };

  const handleAddChannel = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newChannelName.trim()) return;
    if (isTauri) {
      invoke('create_channel', { name: newChannelName, topic: newChannelTopic || 'General', bridgeProtocol: 'Matrix' }).then((c) => { setChannels((p) => [...p, c as ChannelInfo]); setNewChannelName(''); setNewChannelTopic(''); setShowAddChannel(false); }).catch(console.error);
    } else {
      setChannels((p) => [...p, { id: `chan_${Date.now()}`, name: newChannelName, topic: newChannelTopic || 'General', bridge_protocol: 'Matrix', member_count: 1, event_count: 0 }]);
      logLocal('CHANNEL_CREATED', `Federated channel '${newChannelName}' created on Matrix`);
      setNewChannelName(''); setNewChannelTopic(''); setShowAddChannel(false);
    }
  };

  const handleSubscribeDaemon = (channel: string | null) => {
    if (isTauri) {
      invoke('subscribe_daemon_events', { channelId: channel }).then(() => { invoke('get_daemon_status').then((r) => setDaemon(r as DaemonStatus)).catch(console.error); invoke('get_activity_log').then((a) => setActivity(a as ActivityEvent[])).catch(console.error); }).catch((e) => setNotice(`Subscribe failed: ${e}`));
    } else {
      setDaemon((p) => ({ ...p, running: true, sessions: Math.max(p.sessions, 1), subscribed_channel: channel }));
      logLocal('DAEMON_SUBSCRIBE', `Desktop session subscribed to channel ${JSON.stringify(channel)} (preview)`);
    }
  };

  const handlePostGroup = (e: React.FormEvent) => {
    e.preventDefault();
    if (!groupPostText.trim()) return;
    if (isTauri) {
      invoke('post_channel_message', { channelId: groupChannelId, content: groupPostText }).then((c) => { const ch = c as ChannelInfo; setChannels((p) => p.map((x) => (x.id === ch.id ? ch : x))); setGroupPostText(''); invoke('get_activity_log').then((a) => setActivity(a as ActivityEvent[])).catch(console.error); }).catch((err) => setNotice(`Group post failed: ${err}`));
    } else {
      setChannels((p) => p.map((x) => (x.id === groupChannelId ? { ...x, event_count: x.event_count + 1 } : x)));
      logLocal('FEDERATED_POST', `Posted to channel ${groupChannelId}: ${groupPostText} (preview)`);
      setGroupPostText('');
    }
  };

  const handleAddMember = () => {
    const contact = contacts.find((c) => c.unified_id === (groupMemberId || contacts[0]?.unified_id));
    if (!contact) return;
    if (isTauri) {
      invoke('add_channel_member', { channelId: groupChannelId, unifiedId: contact.unified_id, displayName: contact.name }).then((c) => { const ch = c as ChannelInfo; setChannels((p) => p.map((x) => (x.id === ch.id ? ch : x))); invoke('get_activity_log').then((a) => setActivity(a as ActivityEvent[])).catch(console.error); }).catch((err) => setNotice(`Add member failed: ${err}`));
    } else {
      setChannels((p) => p.map((x) => (x.id === groupChannelId ? { ...x, member_count: x.member_count + 1 } : x)));
      logLocal('MEMBER_ADDED', `${contact.name} joined channel ${groupChannelId} (preview)`);
    }
  };

  const handleHandover = (channelId: string, currentBridge: string) => {
    const target = handoverTargets[channelId] || PROTOCOLS.find((p) => p !== currentBridge) || 'Matrix';
    if (isTauri) {
      invoke('handover_channel', { channelId, toProtocol: target }).then((c) => { const ch = c as ChannelInfo; setChannels((p) => p.map((x) => (x.id === ch.id ? ch : x))); invoke('get_activity_log').then((a) => setActivity(a as ActivityEvent[])).catch(console.error); }).catch((err) => setNotice(`Bridge migration failed: ${err}`));
    } else {
      setChannels((p) => p.map((x) => (x.id === channelId ? { ...x, bridge_protocol: target } : x)));
      logLocal('BRIDGE_HANDOVER', `Channel ${channelId} migrated to ${target} (preview)`);
    }
  };

  const filteredContacts = contacts.filter((c) => c.name.toLowerCase().includes(searchQuery.toLowerCase()) || c.primary_protocol.toLowerCase().includes(searchQuery.toLowerCase()));
  const capAreas = ['All', ...Array.from(new Set(capabilities.map((c) => c.area)))];
  const shownCaps = capabilities.filter((c) => capFilter === 'All' || c.area === capFilter);
  const connectedCount = bridges.filter((b) => b.connected).length;

  return (
    <div className="flex flex-col h-screen w-screen bg-slate-950 text-slate-100 font-sans overflow-hidden">
      {/* Header: identity, engine health, server + tray controls. No tabs — everything lives on this page. */}
      <header className="h-14 bg-slate-900 border-b border-slate-800 px-4 flex items-center justify-between shrink-0 z-20">
        <div className="flex items-center gap-3 min-w-0">
          <span className="font-extrabold text-base tracking-wide text-white">OmniCore</span>
          <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-slate-800 text-sky-300 border border-slate-700" title="Version is sourced from VERSION.md and the omnicore-ui crate version; keep both in sync on release.">v{metrics.version}</span>
          <div className="hidden md:flex items-center gap-2 px-2.5 py-1 rounded bg-slate-800/80 border border-slate-700/60 text-xs" title="Encrypted database and bridge engine status reported by the Rust core.">
            <span className={`w-2 h-2 rounded-full ${metrics.server_running ? 'bg-emerald-400 animate-pulse' : 'bg-rose-500'}`} />
            <span className="text-slate-300 font-medium truncate max-w-[420px]">{engineStatus}</span>
            <span className="text-slate-500 font-mono text-[11px]">| {metrics.encryption}</span>
          </div>
        </div>
        <div className="flex items-center gap-2 relative">
          <span className="hidden lg:flex items-center gap-1 text-[11px] text-slate-400">{connectedCount}/{bridges.length} bridges up <Tip text="Connected protocol bridges out of all configured bridges (Signal, Matrix, Discord, Simplex, Tox, iMessage, SMS, Nostr). Toggle them in the Bridges panel on the right." /></span>
          <button onClick={handleToggleServer} className={`text-xs px-2.5 py-1.5 rounded font-medium border ${metrics.server_running ? 'bg-emerald-950/50 text-emerald-300 border-emerald-800/60' : 'bg-rose-950/50 text-rose-300 border-rose-800/60'}`} title="Pause or resume the background routing workers. Pausing stops dispatch but keeps the window and history available.">
            {metrics.server_running ? 'Engine active — Pause' : 'Engine paused — Resume'}
          </button>
          <div className="relative">
            <button onClick={() => setShowTrayMenu(!showTrayMenu)} className="px-2.5 py-1.5 rounded bg-slate-800 hover:bg-slate-700 border border-slate-700 text-xs font-semibold" aria-label="System tray menu" title="System tray controls. The native tray icon (Rust/Tauri) offers the same actions when the window is hidden: show/hide, pause/resume workers, probe bridges, quit servers, quit app.">Tray</button>
            {showTrayMenu && (
              <div className="absolute right-0 mt-2 w-64 bg-slate-900 border border-slate-700 rounded-lg shadow-2xl py-1.5 z-50 text-xs">
                <div className="px-3 py-2 border-b border-slate-800"><div className="font-bold">System tray service</div><div className="text-[11px] text-slate-400">Same actions as the native OS tray icon. The native icon is created by Tauri in omnicore-ui/src/main.rs (setup_system_tray) and stays available when this window is hidden.</div></div>
                <button onClick={() => setShowTrayMenu(false)} className="w-full text-left px-3 py-2 hover:bg-slate-800">Show / hide dashboard <span className="text-slate-500">(native tray: left-click icon)</span></button>
                <button onClick={() => { handleToggleServer(); setShowTrayMenu(false); }} className="w-full text-left px-3 py-2 hover:bg-slate-800">{metrics.server_running ? 'Pause routing workers' : 'Resume routing workers'}</button>
                <button onClick={() => { handleProbe(); setShowTrayMenu(false); }} className="w-full text-left px-3 py-2 hover:bg-slate-800">Probe bridges (health check)</button>
                <div className="border-t border-slate-800 my-1" />
                <button onClick={() => { if (isTauri) invoke('toggle_servers').catch(console.error); setMetrics((p) => ({ ...p, server_running: false })); setShowTrayMenu(false); setNotice('Servers stopped from tray menu. Window kept open.'); }} className="w-full text-left px-3 py-2 hover:bg-slate-800 text-amber-300">Quit servers only (keep window)</button>
                <button onClick={() => { if (isTauri) invoke('quit_app'); else setNotice('Quit requested (preview mode: no app to exit).'); setShowTrayMenu(false); }} className="w-full text-left px-3 py-2 text-rose-400 hover:bg-rose-950/50 font-semibold">Quit OmniCore &amp; servers</button>
              </div>
            )}
          </div>
        </div>
      </header>

      {notice && <div className="bg-sky-950/60 border-b border-sky-900 text-sky-200 text-xs px-4 py-1.5 flex justify-between shrink-0"><span>{notice}</span><button onClick={() => setNotice('')} aria-label="Dismiss">x</button></div>}

      {/* Main row: People | Conversation | Bridges/Routing/Activity */}
      <div className="flex-1 flex overflow-hidden min-h-0">
        <aside className="w-72 bg-slate-900/80 border-r border-slate-800 flex flex-col shrink-0 overflow-hidden">
          <Section title="People" tip="Unified identities: one card per person, grouping their accounts on Signal, Matrix, Discord, Simplex, Tox, iMessage, SMS, and Nostr. Pick a person, then send — OmniCore routes via their primary network and falls back automatically." action={<button onClick={() => setShowAddContact(!showAddContact)} className="text-[11px] px-2 py-1 bg-sky-700 hover:bg-sky-600 rounded font-semibold" title="Create a unified contact and choose its primary and fallback networks.">+ Person</button>}>
            <label className="sr-only" htmlFor="contact-search">Search people</label>
            <input id="contact-search" type="text" placeholder="Search name or network…" value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} className="w-full bg-slate-950 border border-slate-800 rounded px-2.5 py-1.5 text-xs placeholder-slate-500 focus:outline-none focus:border-sky-600 mb-2" />
            {showAddContact && (
              <form onSubmit={handleAddContact} className="mb-2 bg-slate-800/70 border border-slate-700 rounded p-2.5 flex flex-col gap-2 text-xs">
                <div className="font-bold text-sky-300">New unified person</div>
                <input type="text" placeholder="Full name / handle" aria-label="Full name or handle" value={newContactName} onChange={(e) => setNewContactName(e.target.value)} required className="bg-slate-900 border border-slate-700 rounded p-1.5" />
                <div className="grid grid-cols-2 gap-2">
                  <label className="flex flex-col gap-1"><span className="text-[10px] text-slate-400">Primary network <Tip text="First network tried for this person." /></span><select value={newContactPrimary} onChange={(e) => setNewContactPrimary(e.target.value)} className="bg-slate-900 border border-slate-700 rounded p-1">{PROTOCOLS.map((p) => <option key={p}>{p}</option>)}</select></label>
                  <label className="flex flex-col gap-1"><span className="text-[10px] text-slate-400">Fallback network <Tip text="Used automatically when the primary times out or fails." /></span><select value={newContactFallback} onChange={(e) => setNewContactFallback(e.target.value)} className="bg-slate-900 border border-slate-700 rounded p-1">{PROTOCOLS.map((p) => <option key={p}>{p}</option>)}</select></label>
                </div>
                <div className="flex gap-2"><button type="submit" className="flex-1 bg-sky-700 hover:bg-sky-600 rounded py-1 font-semibold">Save person</button><button type="button" onClick={() => setShowAddContact(false)} className="px-2 bg-slate-700 rounded">Cancel</button></div>
              </form>
            )}
            <div className="overflow-y-auto -mx-1 px-1">
              {filteredContacts.map((c) => (
                <div key={c.id} onClick={() => { setSelectedContact(c); setPrimaryProtocol(c.primary_protocol); setFallbackProtocol(c.fallback_protocol); }} className={`p-2.5 flex items-center gap-3 cursor-pointer rounded mb-1 ${selectedContact.id === c.id ? 'bg-sky-950/60 border border-sky-800/60' : 'hover:bg-slate-800/50 border border-transparent'}`}>
                  <div className="w-9 h-9 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center font-bold text-xs text-sky-300 shrink-0">{c.avatar_initials}</div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between gap-2"><span className="font-semibold text-xs truncate">{c.name}</span><span className="text-[10px] text-slate-400 flex items-center gap-1"><span className={`w-1.5 h-1.5 rounded-full ${c.status === 'Online' ? 'bg-emerald-400' : c.status === 'Away' ? 'bg-amber-400' : 'bg-slate-500'}`} />{c.status}</span></div>
                    <div className="flex gap-1 mt-1 flex-wrap">{c.mapped_protocols.map((p) => <span key={p} className="text-[9px] px-1.5 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-300" title={`This person can be reached on ${p} (unified ID ${c.unified_id}).`}>{p}</span>)}</div>
                    <div className="text-[10px] text-slate-500 mt-0.5">Prefers {c.primary_protocol} → falls back to {c.fallback_protocol}</div>
                  </div>
                </div>
              ))}
              {filteredContacts.length === 0 && <p className="text-xs text-slate-500 py-3">No people match “{searchQuery}”.</p>}
            </div>
          </Section>
          <div className="mt-auto px-4 py-3 text-[11px] text-slate-500 leading-snug">Highest-value flow: pick a person, write, send. Everything else on this page supports that delivery. <Tip text="Consolidation note: Bridges, Routing, and Activity used to be separate tab pages. They now live in the right rail of this single page so delivery state is visible while you chat." /></div>
        </aside>

        <main className="flex-1 flex flex-col bg-slate-950 min-w-0">
          <div className="h-14 border-b border-slate-800 px-5 flex items-center justify-between bg-slate-900/40 shrink-0">
            <div className="flex items-center gap-3 min-w-0">
              <div className="w-8 h-8 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center text-xs font-bold text-sky-300">{selectedContact.avatar_initials}</div>
              <div className="min-w-0"><div className="flex items-center gap-2"><span className="font-bold text-sm truncate">{selectedContact.name}</span><span className="text-[10px] text-slate-400 font-mono bg-slate-800/80 px-1.5 py-0.5 rounded">{selectedContact.unified_id}</span></div>
              <div className="text-[11px] text-slate-400">Preferred: <strong className="text-emerald-400">{selectedContact.primary_protocol}</strong> · Fallback: <strong className="text-sky-300">{selectedContact.fallback_protocol}</strong></div></div>
            </div>
            <span className="hidden xl:flex items-center gap-1 text-[11px] bg-slate-800 border border-slate-700 px-2 py-1 rounded text-slate-300">Smart multi-path on <Tip text="The dispatcher tries the primary network, watches for timeouts/receipts, and cascades to fallbacks using the active routing profile (right rail). Circuit breakers skip networks that are down." /></span>
          </div>

          <div className="flex-1 overflow-y-auto p-5 space-y-4">
            {messages.filter((m) => m.recipient === selectedContact.name || m.sender === selectedContact.name).map((msg) => (
              <div key={msg.id} className={`flex flex-col space-y-1 ${msg.is_outgoing ? 'items-end' : 'items-start'}`}>
                <div className="flex items-center gap-2 px-1"><span className="text-[11px] font-bold text-slate-400">{msg.sender}</span><span className="text-[10px] text-slate-500">{msg.timestamp}</span><span className="text-[9px] px-1.5 py-0.5 rounded bg-slate-800 border border-slate-700 text-sky-300" title="Network(s) that carried this message. A fallback note means the primary failed or timed out and OmniCore re-routed automatically.">{msg.protocol}</span></div>
                <div className={`p-3.5 rounded-lg max-w-xl text-sm leading-relaxed ${msg.is_outgoing ? 'bg-sky-700 text-white' : 'bg-slate-800 border border-slate-700 text-slate-100'}`}>{msg.content}</div>
                {msg.is_outgoing && <span className="text-[9px] text-slate-500 pr-1">Status: {msg.status}</span>}
              </div>
            ))}
          </div>

          <div className="p-4 border-t border-slate-800 bg-slate-900/60 shrink-0">
            <div className="flex flex-wrap items-center gap-2 text-xs mb-2">
              <label className="flex items-center gap-1.5"><span className="text-slate-400 font-semibold text-[10px] uppercase tracking-wide">Send via</span><select value={primaryProtocol} onChange={(e) => setPrimaryProtocol(e.target.value)} className="bg-slate-800 border border-slate-700 rounded px-2 py-1 text-emerald-300" title="Primary network for the next message.">{PROTOCOLS.map((p) => <option key={p} value={p}>{p}</option>)}</select></label>
              <span className="text-slate-600">→</span>
              <label className="flex items-center gap-1.5"><span className="text-slate-400 font-semibold text-[10px] uppercase tracking-wide">Fallback</span><select value={fallbackProtocol} onChange={(e) => setFallbackProtocol(e.target.value)} className="bg-slate-800 border border-slate-700 rounded px-2 py-1 text-sky-300" title="Automatic fallback if the primary network times out or rejects delivery.">{PROTOCOLS.map((p) => <option key={p} value={p}>{p}</option>)}</select></label>
              <label className="flex items-center gap-1.5 ml-auto"><span className="text-slate-400 font-semibold text-[10px] uppercase tracking-wide">Routing profile</span><select value={activeProfile} onChange={(e) => handleProfileChange(e.target.value)} className="bg-slate-800 border border-slate-700 rounded px-2 py-1" title="How aggressively OmniCore retries, races, and cascades across networks. Full profile list and explanations are in the Routing panel (right rail).">{profiles.map((p) => <option key={p.id} value={p.id}>{p.label}</option>)}</select> <Tip text="Profiles map to MultipathRouter strategies in omnicore-engine/src/router.rs. Optimal cascade is the safe default; hedged racing trades bandwidth for speed; confirmed receipt waits for signed acknowledgements." /></label>
            </div>
            <form onSubmit={handleSendMessage} className="flex items-center gap-2">
              <input type="text" placeholder={`Message ${selectedContact.name} via ${primaryProtocol}…`} aria-label={`Message ${selectedContact.name}`} value={inputText} onChange={(e) => setInputText(e.target.value)} className="flex-1 bg-slate-950 border border-slate-700 rounded-lg p-3 text-sm placeholder-slate-500 focus:outline-none focus:border-sky-600" />
              <button type="button" onClick={() => setNotice('Attachments: engine stores files in the SQLCipher attachments table; pick a file in the desktop app build.')} className="px-3 py-3 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-xs" title="Attach a file. Attachments are encrypted at rest (attachments table) and fragmented automatically for small-MTU networks like SMS.">Attach</button>
              <button type="submit" className="px-6 py-3 bg-sky-700 hover:bg-sky-600 rounded-lg font-semibold text-sm">Send</button>
            </form>
          </div>
        </main>

        <aside className="w-[380px] bg-slate-900/80 border-l border-slate-800 overflow-y-auto shrink-0">
          <Section title={`Bridges (${connectedCount}/${bridges.length} connected)`} tip="A bridge connects OmniCore to one network (daemon socket, homeserver, gateway, modem, or relay pool). Priority is the fallback order: lower tries first. Latency is the last measured round-trip." action={<button onClick={handleProbe} className="text-[11px] px-2 py-1 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded" title="Send heartbeat probes to open/half-open circuits and log the result in Activity.">Probe all</button>}>
            <div className="flex flex-col gap-2">
              {bridges.map((b) => (
                <div key={b.protocol} className="bg-slate-950/60 border border-slate-800 rounded p-2.5">
                  <div className="flex items-center justify-between gap-2"><span className="font-semibold text-xs">{b.name}</span><Pill ok={b.connected}>{b.status}</Pill></div>
                  <div className="text-[11px] font-mono text-slate-500 truncate" title={b.endpoint}>{b.endpoint}</div>
                  <div className="flex items-center justify-between mt-1.5 text-[11px] text-slate-400"><span>{b.latency_ms} ms · priority #{b.fallback_priority} <Tip text={`Fallback priority ${b.fallback_priority}: when several networks can reach a person, lower numbers are tried earlier in the cascade.`} /></span>
                  <button onClick={() => handleToggleBridge(b.protocol)} className={`px-2 py-0.5 rounded font-semibold border ${b.connected ? 'bg-rose-950/60 text-rose-300 border-rose-800' : 'bg-emerald-950/60 text-emerald-300 border-emerald-800'}`} title={b.connected ? `Disconnect the ${b.protocol} bridge. Messages will cascade to other networks.` : `Connect the ${b.protocol} bridge so it can send and receive.`}>{b.connected ? 'Disconnect' : 'Connect'}</button></div>
                </div>
              ))}
            </div>
          </Section>

          <Section title="Routing" tip="Routing decides how a message finds its person across networks: cascade order, retry policy, circuit breakers, dedup, and receipt checks. Choose one active profile; the capability list below (Engine panel) explains every strategy the engine implements.">
            <div className="flex flex-col gap-1.5">
              {profiles.map((p) => (
                <label key={p.id} className={`flex gap-2 p-2 rounded border cursor-pointer ${activeProfile === p.id ? 'bg-sky-950/50 border-sky-800/70' : 'bg-slate-950/50 border-slate-800 hover:border-slate-700'}`} title={p.engine_symbol}>
                  <input type="radio" name="routing-profile" checked={activeProfile === p.id} onChange={() => handleProfileChange(p.id)} className="mt-0.5 accent-sky-600" />
                  <span><span className="text-xs font-semibold block">{p.label}</span><span className="text-[11px] text-slate-400 leading-snug block">{p.description}</span><span className="text-[10px] font-mono text-slate-600">{p.engine_symbol}</span></span>
                </label>
              ))}
            </div>
          </Section>

          <Section title="Activity" tip="Live ledger of dispatch, fallback, sync, probe, and server events from the Rust core (get_activity_log). New sends and bridge toggles appear here first.">
            <div className="flex flex-col divide-y divide-slate-800/70 font-mono text-[11px] max-h-64 overflow-y-auto">
              {activity.map((a) => <div key={a.id} className="py-2 flex gap-2"><span className="text-emerald-400 shrink-0">[{a.kind}]</span><span className="text-slate-300 leading-snug">{a.detail}</span><span className="text-slate-600 ml-auto shrink-0">{a.timestamp}</span></div>)}
              {activity.length === 0 && <p className="text-slate-500 py-2 font-sans text-xs">No events yet. Send a message or probe bridges.</p>}
            </div>
          </Section>
        </aside>
      </div>

      {/* Bottom band: Security | Groups, mesh & calls | Engine capabilities — all formerly separate pages, now one scroll away. */}
      <div className="h-[300px] shrink-0 border-t border-slate-800 bg-slate-900/60 grid grid-cols-1 lg:grid-cols-3 overflow-hidden">
        <div className="overflow-y-auto border-r border-slate-800">
          <Section title="Security & privacy" tip="Your cryptographic identity, forward-secret sessions, encrypted backup, and optional Tor routing. Fingerprints let contacts verify they are really talking to you.">
            <dl className="text-xs space-y-2">
              <div><dt className="text-slate-500 text-[10px] uppercase tracking-wide">Identity <Tip text="A signed identity card (IdentityCard) binds your unified ID to your per-network accounts. Export it or share the QR payload so contacts can verify you." /></dt><dd className="font-semibold">{security.identity_name} <span className="text-slate-500 font-mono">({security.unified_id})</span> · <Pill ok={security.identity_verified}>{security.identity_verified ? 'Signature verified' : 'Unverified'}</Pill></dd><dd className="font-mono text-[11px] text-slate-400 break-all">{security.fingerprint}</dd></div>
              <div><dt className="text-slate-500 text-[10px] uppercase tracking-wide">Storage encryption</dt><dd>{security.encryption}</dd></div>
              <div><dt className="text-slate-500 text-[10px] uppercase tracking-wide">Session encryption <Tip text="Double Ratchet gives forward secrecy: past messages stay safe even if a key later leaks. Pre-key bundles let people start encrypted chats while you are offline." /></dt><dd className="text-slate-300 leading-snug">{security.ratchet_description}</dd></div>
              <div><dt className="text-slate-500 text-[10px] uppercase tracking-wide">Encrypted backup</dt><dd className="text-slate-300 leading-snug">{security.backup_description}</dd></div>
              <div className="flex items-center justify-between gap-2 pt-1"><span>Tor routing <span className="text-slate-500">({security.tor_proxy_url})</span> <Tip text="Routes bridge traffic through Tor SOCKS5 to reduce ISP/carrier correlation. Status comes from TorManager; enabling here asks the engine to establish a 3-hop circuit." /></span><button onClick={handleToggleTor} className={`px-2 py-1 rounded font-semibold border ${security.tor_enabled ? 'bg-emerald-950/60 text-emerald-300 border-emerald-800' : 'bg-slate-800 text-slate-300 border-slate-700'}`}>{security.tor_enabled ? 'On' : 'Off'}</button></div>
              <div className="text-[11px] text-slate-500">Tor status: {security.tor_status}</div>
              <button onClick={handleExportIdentity} className="w-full mt-1 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded font-semibold" title="Download/copy your signed identity card JSON for pairing another device or sharing with a contact.">Export identity card (JSON)</button>
            </dl>
          </Section>
        </div>

        <div className="overflow-y-auto border-r border-slate-800">
          <Section title="Groups, mesh & calls" tip="Federated group channels replicate across networks with vector clocks. Mesh relays messages device-to-device when the internet is gone. Calls bridge WebRTC, Matrix VoIP, and Discord voice." action={<button onClick={() => setShowAddChannel(!showAddChannel)} className="text-[11px] px-2 py-1 bg-sky-700 hover:bg-sky-600 rounded font-semibold" title="Create a federated group channel hosted on a bridge network.">+ Group</button>}>
            {showAddChannel && <form onSubmit={handleAddChannel} className="mb-2 bg-slate-800/70 border border-slate-700 rounded p-2.5 flex flex-col gap-2 text-xs"><input placeholder="Group name (e.g. family)" aria-label="Group name" value={newChannelName} onChange={(e) => setNewChannelName(e.target.value)} required className="bg-slate-900 border border-slate-700 rounded p-1.5" /><input placeholder="Topic (optional)" aria-label="Group topic" value={newChannelTopic} onChange={(e) => setNewChannelTopic(e.target.value)} className="bg-slate-900 border border-slate-700 rounded p-1.5" /><div className="flex gap-2"><button type="submit" className="flex-1 bg-sky-700 rounded py-1 font-semibold">Create group</button><button type="button" onClick={() => setShowAddChannel(false)} className="px-2 bg-slate-700 rounded">Cancel</button></div></form>}
            <div className="flex flex-col gap-2 text-xs">
              {channels.map((ch) => <div key={ch.id} className="bg-slate-950/60 border border-slate-800 rounded p-2.5"><div className="flex justify-between"><span className="font-semibold">#{ch.name}</span><span className="text-slate-500">{ch.bridge_protocol}</span></div><div className="text-slate-400 text-[11px]">{ch.topic}</div><div className="text-slate-500 text-[11px]">{ch.member_count} members · {ch.event_count} events <Tip text="Members and events replicate with vector-clock causal ordering; the delivery ledger tracks who has confirmed receipt and retries only missing members." /></div><div className="flex gap-1.5 mt-1.5"><select value={handoverTargets[ch.id] || PROTOCOLS.find((p) => p !== ch.bridge_protocol)} onChange={(e) => setHandoverTargets((p) => ({ ...p, [ch.id]: e.target.value }))} aria-label={`Migrate ${ch.name} to bridge`} className="flex-1 min-w-0 bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 text-[11px]" title="Target bridge for a signed channel migration (handover). The old bridge becomes the fallback and causal history is preserved.">{PROTOCOLS.filter((p) => p !== ch.bridge_protocol).map((p) => <option key={p} value={p}>{p}</option>)}</select><button type="button" onClick={() => handleHandover(ch.id, ch.bridge_protocol)} className="px-2 py-0.5 bg-slate-800 border border-slate-700 rounded font-semibold text-[11px]" title="Migrate this channel to the selected bridge with a signed handover (FederatedChannelBridgeHandover).">Migrate</button></div></div>)}
              <div className="flex gap-1.5 text-[11px]">
                <select value={groupMemberId || contacts[0]?.unified_id || ''} onChange={(e) => setGroupMemberId(e.target.value)} aria-label="Person to add" className="flex-1 min-w-0 bg-slate-950 border border-slate-700 rounded px-1.5 py-1" title="Person (unified contact) to add to the selected channel below. Adding is idempotent.">{contacts.map((c) => <option key={c.unified_id} value={c.unified_id}>{c.name}</option>)}</select>
                <button type="button" onClick={handleAddMember} className="px-2 py-1 bg-slate-800 border border-slate-700 rounded font-semibold" title="Add the selected person as a member of the selected group channel so posts broadcast to them.">Add member</button>
              </div>
              <form onSubmit={handlePostGroup} className="flex gap-1.5 text-[11px]">
                <select value={groupChannelId} onChange={(e) => setGroupChannelId(e.target.value)} aria-label="Group channel" className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1 max-w-[110px]" title="Channel to post into. The event is recorded in its vector-clock history and broadcast to other members via the router.">{channels.map((ch) => <option key={ch.id} value={ch.id}>#{ch.name}</option>)}</select>
                <input value={groupPostText} onChange={(e) => setGroupPostText(e.target.value)} placeholder="Post to group…" aria-label="Group message" className="flex-1 min-w-0 bg-slate-950 border border-slate-700 rounded px-2 py-1" />
                <button type="submit" className="px-2 py-1 bg-sky-700 hover:bg-sky-600 rounded font-semibold" title="Record this post in the channel's causal event history and broadcast it to other members.">Post</button>
              </form>
              <p className="text-[11px] text-slate-400 leading-snug"><strong className="text-slate-300">Federation:</strong> {network.federation_description} ({network.federation_channels} group{network.federation_channels === 1 ? '' : 's'} here)</p>
              <p className="text-[11px] text-slate-400 leading-snug"><strong className="text-slate-300">Mesh:</strong> {network.mesh_description} Peers nearby: {network.mesh_peers}. <strong className="text-slate-300">Discovery:</strong> {network.discovery_description}</p>
              <p className="text-[11px] text-slate-400 leading-snug"><strong className="text-slate-300">Calls:</strong> {network.call_description} Active calls: {network.active_calls}.</p>
              <p className="text-[11px] text-slate-400 leading-snug"><strong className="text-slate-300">Anti-spam:</strong> {network.reputation_description}</p>
            </div>
          </Section>
        </div>

        <div className="overflow-y-auto">
          <Section title="Engine & system" tip="Complete map of what the Rust engine implements, grouped by area. If a feature exists in omnicore-engine, it is listed here with the exact type/function name. Use the filter to audit one area.">
            <div className="flex gap-1 flex-wrap mb-2">{capAreas.map((a) => <button key={a} onClick={() => setCapFilter(a)} className={`text-[10px] px-2 py-0.5 rounded border ${capFilter === a ? 'bg-sky-800 text-white border-sky-700' : 'bg-slate-800 text-slate-400 border-slate-700'}`}>{a}</button>)}</div>
            <p className="text-[11px] text-slate-400 leading-snug mb-1"><strong className="text-slate-300">Device:</strong> {network.telemetry_summary}. Appliance action: <Pill ok={network.appliance_action === 'Normal'}>{network.appliance_action}</Pill> <Tip text="Appliance policy (ApplianceConfig) can trigger LTE failover on power loss and safe shutdown on heat/low battery. Telemetry here is from TelemetryCollector." /></p>
            <p className="text-[11px] text-slate-400 leading-snug mb-1"><strong className="text-slate-300">Offline queue:</strong> {network.outbox_description} Queued now: {network.outbox_queued}.</p>
            <p className="text-[11px] text-slate-400 leading-snug mb-2"><strong className="text-slate-300">Daemon:</strong> {daemon.url} · {daemon.running ? 'running' : 'stopped'} · {daemon.sessions} client session(s) <Tip text="Desktop, mobile, and CLI clients talk to the headless engine over this WebSocket JSON-RPC daemon (server.rs). Subscribe this desktop session to a channel to receive channel-targeted message_dispatched events; sends broadcast to subscribers and log DAEMON_EVENT in Activity. Mobile uses the same endpoint in its Settings screen." /></p>
            <div className="flex items-center gap-1.5 mb-2 text-[11px]">
              <input value={subChannel} onChange={(e) => setSubChannel(e.target.value)} placeholder="Channel (e.g. ops-room)" aria-label="Daemon channel" className="flex-1 min-w-0 bg-slate-950 border border-slate-700 rounded px-2 py-1" />
              <button onClick={() => handleSubscribeDaemon(subChannel || null)} className="px-2 py-1 bg-sky-700 hover:bg-sky-600 rounded font-semibold" title="Subscribe this session to the channel via the daemon SubscribeEvents RPC.">Subscribe</button>
              <button onClick={() => handleSubscribeDaemon(null)} className="px-2 py-1 bg-slate-800 border border-slate-700 rounded" title="Clear the channel subscription (global events only).">Clear</button>
            </div>
            <p className="text-[11px] text-slate-500 mb-2">Subscribed channel: {daemon.subscribed_channel ?? 'none (global events)'}</p>
            <div className="flex flex-col gap-1.5">
              {shownCaps.map((c) => <div key={c.name} className="bg-slate-950/60 border border-slate-800 rounded p-2"><div className="flex justify-between gap-2"><span className="text-xs font-semibold">{c.name}</span><span className="text-[9px] text-slate-500 uppercase tracking-wide shrink-0">{c.area}</span></div><p className="text-[11px] text-slate-400 leading-snug">{c.description}</p><p className="text-[10px] font-mono text-slate-600 break-all">{c.engine_symbol} · {c.status}</p></div>)}
            </div>
          </Section>
        </div>
      </div>

      <footer className="h-7 bg-slate-900 border-t border-slate-800 px-4 flex items-center justify-between text-[11px] text-slate-500 shrink-0">
        <span>OmniCore v{metrics.version} · {metrics.encryption} · Uptime {Math.floor(metrics.uptime_secs / 60)}m {metrics.uptime_secs % 60}s</span>
        <span>{connectedCount} bridges connected · Profile: {activeProfile} · Tor {security.tor_enabled ? 'on' : 'off'} · Single-page dashboard (former Bridges / Routing pages merged here)</span>
      </footer>
    </div>
  );
}

