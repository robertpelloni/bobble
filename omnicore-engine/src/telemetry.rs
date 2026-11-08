use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerSource {
    AcMains,
    Battery,
    Ups,
    SolarPoE,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryStatus {
    pub percentage: u8,
    pub is_charging: bool,
    pub voltage_volts: f32,
    pub temperature_celsius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareHealth {
    pub cpu_temp_celsius: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_free_gb: f32,
    pub is_throttled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTelemetry {
    pub power_source: PowerSource,
    pub battery: Option<BatteryStatus>,
    pub health: HardwareHealth,
    pub cellular_signal_dbm: Option<i32>,
    pub wifi_signal_dbm: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplianceAction {
    Normal,
    Throttled,
    CellularFallback,
    SafeShutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplianceConfig {
    pub power_failover_enabled: bool,
    pub cellular_hat_device_path: String,
    pub thermal_cutoff_celsius: f32,
    pub low_battery_cutoff_pct: u8,
}

impl Default for ApplianceConfig {
    fn default() -> Self {
        Self {
            power_failover_enabled: true,
            cellular_hat_device_path: "/dev/ttyUSB0".to_string(),
            thermal_cutoff_celsius: 85.0,
            low_battery_cutoff_pct: 10,
        }
    }
}

impl ApplianceConfig {
    pub fn evaluate_action(&self, telemetry: &SystemTelemetry) -> ApplianceAction {
        // Critical thermal emergency
        if telemetry.health.cpu_temp_celsius >= self.thermal_cutoff_celsius {
            return ApplianceAction::SafeShutdown;
        }

        // Critical battery emergency
        if let Some(ref batt) = telemetry.battery {
            if batt.percentage <= self.low_battery_cutoff_pct && !batt.is_charging {
                return ApplianceAction::SafeShutdown;
            }
        }

        // Cellular failover when power is on UPS/Battery and WiFi is lost
        if self.power_failover_enabled
            && (telemetry.power_source == PowerSource::Battery || telemetry.power_source == PowerSource::Ups)
            && telemetry.wifi_signal_dbm.is_none()
        {
            return ApplianceAction::CellularFallback;
        }

        if telemetry.health.is_throttled || telemetry.health.cpu_temp_celsius >= 75.0 {
            ApplianceAction::Throttled
        } else {
            ApplianceAction::Normal
        }
    }
}

pub struct TelemetryCollector {
    mock_mode: bool,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self { mock_mode: true }
    }

    pub fn collect(&self) -> SystemTelemetry {
        if self.mock_mode {
            SystemTelemetry {
                power_source: PowerSource::AcMains,
                battery: Some(BatteryStatus {
                    percentage: 98,
                    is_charging: true,
                    voltage_volts: 12.6,
                    temperature_celsius: 28.5,
                }),
                health: HardwareHealth {
                    cpu_temp_celsius: 42.0,
                    memory_used_mb: 256,
                    memory_total_mb: 4096,
                    disk_free_gb: 28.4,
                    is_throttled: false,
                },
                cellular_signal_dbm: Some(-75),
                wifi_signal_dbm: Some(-52),
            }
        } else {
            // Sysfs and hwmon read fallback
            SystemTelemetry {
                power_source: PowerSource::AcMains,
                battery: None,
                health: HardwareHealth {
                    cpu_temp_celsius: 40.0,
                    memory_used_mb: 128,
                    memory_total_mb: 2048,
                    disk_free_gb: 10.0,
                    is_throttled: false,
                },
                cellular_signal_dbm: None,
                wifi_signal_dbm: None,
            }
        }
    }

    pub fn is_power_critical(&self, telemetry: &SystemTelemetry) -> bool {
        if let Some(battery) = &telemetry.battery {
            if !battery.is_charging && battery.percentage < 15 {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_collection() {
        let collector = TelemetryCollector::new();
        let telemetry = collector.collect();

        assert_eq!(telemetry.power_source, PowerSource::AcMains);
        assert!(telemetry.battery.is_some());
        assert_eq!(telemetry.battery.as_ref().unwrap().percentage, 98);
        assert!(!collector.is_power_critical(&telemetry));
    }

    #[test]
    fn test_power_critical_threshold() {
        let collector = TelemetryCollector::new();
        let mut telemetry = collector.collect();

        // Simulate battery discharging below 15%
        telemetry.power_source = PowerSource::Battery;
        if let Some(bat) = &mut telemetry.battery {
            bat.is_charging = false;
            bat.percentage = 12;
        }

        assert!(collector.is_power_critical(&telemetry));
    }

    #[test]
    fn test_appliance_cellular_failover_action() {
        let config = ApplianceConfig::default();
        let telemetry = SystemTelemetry {
            power_source: PowerSource::Battery,
            battery: Some(BatteryStatus {
                percentage: 60,
                is_charging: false,
                voltage_volts: 12.2,
                temperature_celsius: 28.0,
            }),
            health: HardwareHealth {
                cpu_temp_celsius: 45.0,
                memory_used_mb: 200,
                memory_total_mb: 1024,
                disk_free_gb: 12.0,
                is_throttled: false,
            },
            cellular_signal_dbm: Some(-75),
            wifi_signal_dbm: None, // WAN disconnected
        };

        assert_eq!(config.evaluate_action(&telemetry), ApplianceAction::CellularFallback);
    }

    #[test]
    fn test_appliance_safe_shutdown_action() {
        let config = ApplianceConfig::default();
        let emergency_temp = SystemTelemetry {
            power_source: PowerSource::AcMains,
            battery: None,
            health: HardwareHealth {
                cpu_temp_celsius: 88.0, // Exceeds 85.0 cutoff
                memory_used_mb: 300,
                memory_total_mb: 1024,
                disk_free_gb: 10.0,
                is_throttled: true,
            },
            cellular_signal_dbm: None,
            wifi_signal_dbm: Some(-50),
        };

        assert_eq!(config.evaluate_action(&emergency_temp), ApplianceAction::SafeShutdown);
    }
}
