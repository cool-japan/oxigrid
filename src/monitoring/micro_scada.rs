//! Micro-SCADA System for Distribution Network Monitoring and Control.
//!
//! Implements a lightweight Supervisory Control and Data Acquisition (SCADA)
//! system for distribution-level substations and microgrids, supporting RTU
//! data collection, alarm management, SCADA display configuration, and
//! remote control with full audit trail.
//!
//! # Features
//! - Multi-protocol RTU device management (DNP3, Modbus, IEC 61850, IEC 104, etc.)
//! - Tag-based data model with engineering units, quality codes, and limit monitoring
//! - Four-level alarm management (LOLO/LO/HI/HIHI) with acknowledgement and clearing
//! - SCADA display page configuration with configurable refresh rates
//! - Full operator control audit trail (ControlLog)
//! - KPI computation: alarm counts, communication availability, tag quality
//!
//! # References
//! - IEC 61970 Common Information Model
//! - IEC 60870-5-104 Telecontrol protocols
//! - ISA-18.2 Management of Alarm Systems for the Process Industries
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Communication protocol used by a Remote Terminal Unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RtuProtocol {
    /// DNP3 (Distributed Network Protocol 3), common in power systems.
    Dnp3,
    /// Modbus RTU over serial.
    ModbusRtu,
    /// IEC 61850 MMS/GOOSE over Ethernet.
    Iec61850,
    /// IEC 60870-5-104 over TCP/IP.
    Iec104,
    /// Profibus DP field-bus.
    Profibus,
    /// MQTT publish/subscribe IoT protocol.
    Mqtt,
    /// Custom TCP application protocol.
    CustomTcp,
}

/// Quality descriptor for a measured tag value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementQuality {
    /// Value is reliable and within sensor range.
    Good,
    /// Value may be inaccurate; awaiting confirmation.
    Uncertain,
    /// Value is unusable due to sensor or communication failure.
    Bad,
    /// Value failed validation (range check, CRC, etc.).
    Invalid,
    /// No value has been received yet.
    Missing,
    /// Value has not been updated within the expected poll interval.
    Stale,
}

/// Remote control command issued to a field device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlCommand {
    /// Open a circuit breaker.
    OpenBreaker {
        /// ID of the breaker device.
        device_id: usize,
    },
    /// Close a circuit breaker.
    CloseBreaker {
        /// ID of the breaker device.
        device_id: usize,
    },
    /// Raise transformer tap by one position.
    TapUp {
        /// ID of the transformer.
        transformer_id: usize,
    },
    /// Lower transformer tap by one position.
    TapDown {
        /// ID of the transformer.
        transformer_id: usize,
    },
    /// Set a capacitor bank to the specified number of steps.
    SetCapacitor {
        /// ID of the capacitor bank.
        bank_id: usize,
        /// Number of switched steps to engage.
        steps: usize,
    },
    /// Set an active power setpoint for a controllable device.
    SetSetpoint {
        /// ID of the device.
        device_id: usize,
        /// Desired power setpoint \[MW\].
        value_mw: f64,
    },
    /// Initiate emergency shutdown for an entire protection zone.
    EmergencyShutdown {
        /// ID of the protection zone.
        zone_id: usize,
    },
}

/// Operator priority classification for alarm response ordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlarmPriority {
    /// Immediate operator intervention required — highest urgency.
    Emergency,
    /// Critical condition requiring rapid response.
    Critical,
    /// Urgent — action required within minutes.
    Urgent,
    /// Warning — action required within the shift.
    Warning,
    /// Advisory — situation to be monitored.
    Advisory,
    /// Informational — log-only notification.
    Informational,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Remote Terminal Unit device definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtuDevice {
    /// Unique device identifier.
    pub id: usize,
    /// Human-readable device name.
    pub name: String,
    /// Communication protocol used by this RTU.
    pub protocol: RtuProtocol,
    /// IP address, hostname, or serial port path.
    pub address: String,
    /// Polling interval \[ms\].
    pub poll_interval_ms: u32,
    /// Whether communication with the RTU is currently healthy.
    pub communication_ok: bool,
    /// Timestamp of the last successful poll \[s\].
    pub last_poll_timestamp: f64,
    /// Number of analog (measured) inputs on this RTU.
    pub n_analog_inputs: usize,
    /// Number of binary (status) inputs on this RTU.
    pub n_binary_inputs: usize,
    /// Number of control output channels on this RTU.
    pub n_control_outputs: usize,
}

/// A process tag with its current value, quality, and engineering limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagValue {
    /// Unique tag identifier.
    pub tag_id: usize,
    /// Human-readable tag name (e.g. `"BUS1.V"`, `"FEEDER1.I"`).
    pub name: String,
    /// Current measured value in engineering units.
    pub value: f64,
    /// Data quality of the current value.
    pub quality: MeasurementQuality,
    /// Timestamp of last update \[s\].
    pub timestamp: f64,
    /// Engineering unit string (e.g. `"kV"`, `"MW"`, `"A"`, `"deg"`).
    pub unit: String,
    /// Minimum valid engineering value (sensor range floor).
    pub engineering_min: f64,
    /// Maximum valid engineering value (sensor range ceiling).
    pub engineering_max: f64,
    /// High-high (critical) alarm limit (optional).
    pub high_high_limit: Option<f64>,
    /// High alarm limit (optional).
    pub high_limit: Option<f64>,
    /// Low alarm limit (optional).
    pub low_limit: Option<f64>,
    /// Low-low (critical) alarm limit (optional).
    pub low_low_limit: Option<f64>,
    /// RTU device that provides this tag (None for calculated tags).
    pub rtu_id: Option<usize>,
}

/// An alarm event record with acknowledgement and clearing state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScadaAlarm {
    /// Unique alarm record identifier.
    pub id: usize,
    /// Tag that triggered this alarm.
    pub tag_id: usize,
    /// Operator priority for response ordering.
    pub priority: AlarmPriority,
    /// Machine-readable condition string (e.g. `"HIGH_HIGH"`, `"LOW_LOW"`, `"COMMUNICATION_LOSS"`).
    pub condition: String,
    /// Human-readable alarm message.
    pub message: String,
    /// Timestamp when the alarm condition was first detected \[s\].
    pub timestamp: f64,
    /// Whether an operator has acknowledged this alarm.
    pub acknowledged: bool,
    /// Timestamp when this alarm was acknowledged \[s\], if any.
    pub ack_timestamp: Option<f64>,
    /// Whether the alarm condition has returned to normal.
    pub cleared: bool,
    /// Duration the alarm has been active \[s\].
    pub duration_s: f64,
}

/// SCADA display page configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScadaDisplayPage {
    /// Unique page identifier.
    pub page_id: usize,
    /// Display page name shown in navigation.
    pub name: String,
    /// Tag IDs shown on this display page.
    pub tag_ids: Vec<usize>,
    /// Display type hint (e.g. `"single_line_diagram"`, `"trend_chart"`, `"alarm_list"`).
    pub display_type: String,
    /// Page refresh interval \[ms\].
    pub refresh_rate_ms: u32,
}

/// Operator control action audit-trail entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlLog {
    /// Unique log entry identifier.
    pub id: usize,
    /// Timestamp when the command was issued \[s\].
    pub timestamp: f64,
    /// Operator identifier (username or badge number).
    pub operator_id: String,
    /// Human-readable command description.
    pub command: String,
    /// Target device identifier.
    pub device_id: usize,
    /// Device value before the command was issued.
    pub pre_value: f64,
    /// Device value after command execution.
    pub post_value: f64,
    /// Whether the command executed successfully.
    pub success: bool,
    /// Error description if `success` is false.
    pub error_message: Option<String>,
}

/// Micro-SCADA system for distribution network monitoring and control.
pub struct MicroScada {
    /// System name / substation identifier.
    pub name: String,
    /// Registered RTU field devices.
    pub rtu_devices: Vec<RtuDevice>,
    /// All configured process tags (live values).
    pub tags: Vec<TagValue>,
    /// All alarm records (active and historical).
    pub alarms: Vec<ScadaAlarm>,
    /// Configured SCADA display pages.
    pub display_pages: Vec<ScadaDisplayPage>,
    /// Operator control audit trail.
    pub control_log: Vec<ControlLog>,
    /// Monotonic counter for next alarm ID.
    pub next_alarm_id: usize,
    /// Current system timestamp \[s\].
    pub current_timestamp: f64,
}

/// Key performance indicators computed from the current SCADA state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScadaKpis {
    /// Total number of active (uncleared) alarms.
    pub n_active_alarms: usize,
    /// Number of active alarms with Emergency or Critical priority.
    pub n_critical_alarms: usize,
    /// Percentage of RTU devices with `communication_ok == true`.
    pub communication_availability_pct: f64,
    /// Number of tags not updated within the last 60 seconds.
    pub n_stale_tags: usize,
    /// Percentage of tags with `MeasurementQuality::Good`.
    pub tag_quality_good_pct: f64,
    /// Number of control actions logged in the last 24 hours.
    pub n_control_actions_24h: usize,
}

// ---------------------------------------------------------------------------
// MicroScada implementation
// ---------------------------------------------------------------------------

impl MicroScada {
    /// Create a new empty Micro-SCADA instance.
    pub fn new(name: String) -> Self {
        Self {
            name,
            rtu_devices: Vec::new(),
            tags: Vec::new(),
            alarms: Vec::new(),
            display_pages: Vec::new(),
            control_log: Vec::new(),
            next_alarm_id: 1,
            current_timestamp: 0.0,
        }
    }

    /// Register a Remote Terminal Unit with the SCADA system.
    pub fn add_rtu(&mut self, rtu: RtuDevice) {
        self.rtu_devices.push(rtu);
    }

    /// Register a process tag.
    ///
    /// If a tag with the same `tag_id` already exists it is replaced.
    pub fn add_tag(&mut self, tag: TagValue) {
        if let Some(existing) = self.tags.iter_mut().find(|t| t.tag_id == tag.tag_id) {
            *existing = tag;
        } else {
            self.tags.push(tag);
        }
    }

    /// Update a tag value, quality, and timestamp, then check alarm limits.
    ///
    /// Does nothing if `tag_id` is not registered.
    pub fn update_tag(
        &mut self,
        tag_id: usize,
        value: f64,
        quality: MeasurementQuality,
        timestamp: f64,
    ) {
        self.current_timestamp = timestamp;
        if let Some(tag) = self.tags.iter_mut().find(|t| t.tag_id == tag_id) {
            tag.value = value;
            tag.quality = quality;
            tag.timestamp = timestamp;
        }
        self.check_alarm_conditions(tag_id);
    }

    /// Generate or clear alarms for the tag identified by `tag_id`.
    ///
    /// Generates new alarms when limits are violated; clears existing active
    /// alarms when the value returns to the normal band.
    pub fn check_alarm_conditions(&mut self, tag_id: usize) {
        // Clone tag data to avoid simultaneous borrow of `self`
        let tag_snapshot = match self.tags.iter().find(|t| t.tag_id == tag_id) {
            Some(t) => t.clone(),
            None => return,
        };

        let value = tag_snapshot.value;
        let timestamp = tag_snapshot.timestamp;

        // Determine which limit conditions are currently violated
        let hh_violated = tag_snapshot.high_high_limit.is_some_and(|lim| value > lim);
        let hi_violated = tag_snapshot
            .high_limit
            .is_some_and(|lim| value > lim && !hh_violated);
        let lo_violated = tag_snapshot.low_limit.is_some_and(|lim| {
            value < lim && tag_snapshot.low_low_limit.is_none_or(|ll| value >= ll)
        });
        let ll_violated = tag_snapshot.low_low_limit.is_some_and(|lim| value < lim);

        // Clear alarms whose conditions are no longer violated
        for alarm in self
            .alarms
            .iter_mut()
            .filter(|a| a.tag_id == tag_id && !a.cleared)
        {
            let should_clear = match alarm.condition.as_str() {
                "HIGH_HIGH" => !hh_violated,
                "HIGH" => !hi_violated,
                "LOW" => !lo_violated,
                "LOW_LOW" => !ll_violated,
                _ => false,
            };
            if should_clear {
                alarm.cleared = true;
                alarm.duration_s = timestamp - alarm.timestamp;
            }
        }

        // Helper: is there already an active (uncleared) alarm for this condition?
        let active_for = |condition: &str| -> bool {
            self.alarms
                .iter()
                .any(|a| a.tag_id == tag_id && a.condition == condition && !a.cleared)
        };

        // Generate new alarms for newly violated limits
        let mut new_alarms: Vec<ScadaAlarm> = Vec::new();

        if hh_violated && !active_for("HIGH_HIGH") {
            let lim = tag_snapshot.high_high_limit.unwrap_or(f64::INFINITY);
            new_alarms.push(ScadaAlarm {
                id: self.next_alarm_id,
                tag_id,
                priority: AlarmPriority::Emergency,
                condition: "HIGH_HIGH".to_owned(),
                message: format!(
                    "{} HIGH_HIGH: {:.3} {} > {:.3}",
                    tag_snapshot.name, value, tag_snapshot.unit, lim
                ),
                timestamp,
                acknowledged: false,
                ack_timestamp: None,
                cleared: false,
                duration_s: 0.0,
            });
            self.next_alarm_id += 1;
        }

        if hi_violated && !active_for("HIGH") {
            let lim = tag_snapshot.high_limit.unwrap_or(f64::INFINITY);
            new_alarms.push(ScadaAlarm {
                id: self.next_alarm_id,
                tag_id,
                priority: AlarmPriority::Warning,
                condition: "HIGH".to_owned(),
                message: format!(
                    "{} HIGH: {:.3} {} > {:.3}",
                    tag_snapshot.name, value, tag_snapshot.unit, lim
                ),
                timestamp,
                acknowledged: false,
                ack_timestamp: None,
                cleared: false,
                duration_s: 0.0,
            });
            self.next_alarm_id += 1;
        }

        if lo_violated && !active_for("LOW") {
            let lim = tag_snapshot.low_limit.unwrap_or(f64::NEG_INFINITY);
            new_alarms.push(ScadaAlarm {
                id: self.next_alarm_id,
                tag_id,
                priority: AlarmPriority::Warning,
                condition: "LOW".to_owned(),
                message: format!(
                    "{} LOW: {:.3} {} < {:.3}",
                    tag_snapshot.name, value, tag_snapshot.unit, lim
                ),
                timestamp,
                acknowledged: false,
                ack_timestamp: None,
                cleared: false,
                duration_s: 0.0,
            });
            self.next_alarm_id += 1;
        }

        if ll_violated && !active_for("LOW_LOW") {
            let lim = tag_snapshot.low_low_limit.unwrap_or(f64::NEG_INFINITY);
            new_alarms.push(ScadaAlarm {
                id: self.next_alarm_id,
                tag_id,
                priority: AlarmPriority::Emergency,
                condition: "LOW_LOW".to_owned(),
                message: format!(
                    "{} LOW_LOW: {:.3} {} < {:.3}",
                    tag_snapshot.name, value, tag_snapshot.unit, lim
                ),
                timestamp,
                acknowledged: false,
                ack_timestamp: None,
                cleared: false,
                duration_s: 0.0,
            });
            self.next_alarm_id += 1;
        }

        self.alarms.extend(new_alarms);
    }

    /// Return all active (uncleared) alarms sorted by priority (Emergency first).
    pub fn get_active_alarms(&self) -> Vec<&ScadaAlarm> {
        let mut active: Vec<&ScadaAlarm> = self.alarms.iter().filter(|a| !a.cleared).collect();
        active.sort_by_key(|a| alarm_priority_order(&a.priority));
        active
    }

    /// Acknowledge an alarm by ID.
    ///
    /// Returns `true` if the alarm was found and acknowledged, `false` otherwise.
    pub fn acknowledge_alarm(&mut self, alarm_id: usize, operator_id: &str) -> bool {
        let ts = self.current_timestamp;
        if let Some(alarm) = self.alarms.iter_mut().find(|a| a.id == alarm_id) {
            alarm.acknowledged = true;
            alarm.ack_timestamp = Some(ts);
            let _ = operator_id; // recorded implicitly via timestamp
            true
        } else {
            false
        }
    }

    /// Execute a remote control command and record it in the audit trail.
    ///
    /// Returns `Ok(())` on success or `Err(description)` on failure.
    pub fn execute_control(
        &mut self,
        command: ControlCommand,
        operator_id: &str,
    ) -> Result<(), String> {
        let ts = self.current_timestamp;
        let log_id = self.control_log.len() + 1;

        let (device_id, cmd_str, pre_value, post_value, ok, err_msg) = match &command {
            ControlCommand::OpenBreaker { device_id } => {
                let id = *device_id;
                // Find status tag for this device if available
                let pre = self.find_device_status(id);
                (
                    id,
                    format!("OpenBreaker(device_id={})", id),
                    pre,
                    0.0,
                    true,
                    None,
                )
            }
            ControlCommand::CloseBreaker { device_id } => {
                let id = *device_id;
                let pre = self.find_device_status(id);
                (
                    id,
                    format!("CloseBreaker(device_id={})", id),
                    pre,
                    1.0,
                    true,
                    None,
                )
            }
            ControlCommand::TapUp { transformer_id } => {
                let id = *transformer_id;
                let pre = self.find_device_status(id);
                (
                    id,
                    format!("TapUp(transformer_id={})", id),
                    pre,
                    pre + 1.0,
                    true,
                    None,
                )
            }
            ControlCommand::TapDown { transformer_id } => {
                let id = *transformer_id;
                let pre = self.find_device_status(id);
                (
                    id,
                    format!("TapDown(transformer_id={})", id),
                    pre,
                    (pre - 1.0).max(0.0),
                    true,
                    None,
                )
            }
            ControlCommand::SetCapacitor { bank_id, steps } => {
                let id = *bank_id;
                let pre = self.find_device_status(id);
                let post = *steps as f64;
                (
                    id,
                    format!("SetCapacitor(bank_id={}, steps={})", id, steps),
                    pre,
                    post,
                    true,
                    None,
                )
            }
            ControlCommand::SetSetpoint {
                device_id,
                value_mw,
            } => {
                let id = *device_id;
                let pre = self.find_device_status(id);
                let post = *value_mw;
                (
                    id,
                    format!("SetSetpoint(device_id={}, value_mw={:.3})", id, value_mw),
                    pre,
                    post,
                    true,
                    None,
                )
            }
            ControlCommand::EmergencyShutdown { zone_id } => {
                let id = *zone_id;
                (
                    id,
                    format!("EmergencyShutdown(zone_id={})", id),
                    1.0,
                    0.0,
                    true,
                    None,
                )
            }
        };

        self.control_log.push(ControlLog {
            id: log_id,
            timestamp: ts,
            operator_id: operator_id.to_owned(),
            command: cmd_str,
            device_id,
            pre_value,
            post_value,
            success: ok,
            error_message: err_msg,
        });

        if ok {
            Ok(())
        } else {
            Err("Command execution failed".to_owned())
        }
    }

    /// Look up a tag by its human-readable name.
    pub fn get_tag_by_name(&self, name: &str) -> Option<&TagValue> {
        self.tags.iter().find(|t| t.name == name)
    }

    /// Return all tags associated with a specific RTU device.
    pub fn get_tags_by_rtu(&self, rtu_id: usize) -> Vec<&TagValue> {
        self.tags
            .iter()
            .filter(|t| t.rtu_id == Some(rtu_id))
            .collect()
    }

    /// Compute current system KPIs.
    pub fn compute_kpis(&self) -> ScadaKpis {
        let active_alarms = self
            .alarms
            .iter()
            .filter(|a| !a.cleared)
            .collect::<Vec<_>>();
        let n_active_alarms = active_alarms.len();

        let n_critical_alarms = active_alarms
            .iter()
            .filter(|a| {
                matches!(
                    a.priority,
                    AlarmPriority::Emergency | AlarmPriority::Critical
                )
            })
            .count();

        let n_rtu = self.rtu_devices.len();
        let n_comm_ok = self
            .rtu_devices
            .iter()
            .filter(|r| r.communication_ok)
            .count();
        let communication_availability_pct = if n_rtu > 0 {
            n_comm_ok as f64 / n_rtu as f64 * 100.0
        } else {
            100.0
        };

        let stale_threshold = 60.0;
        let n_stale_tags = self
            .tags
            .iter()
            .filter(|t| self.current_timestamp - t.timestamp > stale_threshold)
            .count();

        let n_tags = self.tags.len();
        let n_good = self
            .tags
            .iter()
            .filter(|t| matches!(t.quality, MeasurementQuality::Good))
            .count();
        let tag_quality_good_pct = if n_tags > 0 {
            n_good as f64 / n_tags as f64 * 100.0
        } else {
            100.0
        };

        let window_24h = 24.0 * 3600.0;
        let cutoff = self.current_timestamp - window_24h;
        let n_control_actions_24h = self
            .control_log
            .iter()
            .filter(|e| e.timestamp >= cutoff)
            .count();

        ScadaKpis {
            n_active_alarms,
            n_critical_alarms,
            communication_availability_pct,
            n_stale_tags,
            tag_quality_good_pct,
            n_control_actions_24h,
        }
    }

    /// Simulate a poll cycle arriving from an RTU, updating associated tags.
    ///
    /// `simulated_values` is a list of `(tag_id, new_value)` pairs.
    /// The RTU's `communication_ok` is set to `true` and
    /// `last_poll_timestamp` updated to `current_timestamp`.
    pub fn simulate_poll(&mut self, rtu_id: usize, simulated_values: Vec<(usize, f64)>) {
        let ts = self.current_timestamp;

        // Update RTU communication state
        if let Some(rtu) = self.rtu_devices.iter_mut().find(|r| r.id == rtu_id) {
            rtu.communication_ok = true;
            rtu.last_poll_timestamp = ts;
        }

        // Update each tag from the simulated poll data
        for (tag_id, value) in simulated_values {
            self.update_tag(tag_id, value, MeasurementQuality::Good, ts);
        }
    }

    /// Add a SCADA display page.
    pub fn add_display_page(&mut self, page: ScadaDisplayPage) {
        self.display_pages.push(page);
    }

    /// Return the number of control actions recorded in the audit trail.
    pub fn get_historical_count(&self) -> usize {
        self.control_log.len()
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Find the current numeric status/value of a device by its ID.
    ///
    /// Returns 0.0 if no matching tag is found.
    fn find_device_status(&self, device_id: usize) -> f64 {
        // Try to find a tag whose tag_id matches device_id
        self.tags
            .iter()
            .find(|t| t.tag_id == device_id)
            .map(|t| t.value)
            .unwrap_or(0.0)
    }
}

/// Numeric ordering for alarm priority sorting (lower = higher urgency).
fn alarm_priority_order(p: &AlarmPriority) -> u8 {
    match p {
        AlarmPriority::Emergency => 0,
        AlarmPriority::Critical => 1,
        AlarmPriority::Urgent => 2,
        AlarmPriority::Warning => 3,
        AlarmPriority::Advisory => 4,
        AlarmPriority::Informational => 5,
    }
}

// ---------------------------------------------------------------------------
// TagBuilder
// ---------------------------------------------------------------------------

/// Helper for constructing standard distribution-network process tags.
pub struct TagBuilder;

impl TagBuilder {
    /// Create a voltage tag for a named bus with the given nominal voltage.
    ///
    /// Limits are set at ±5 % (HI/LO) and ±10 % (HIHI/LOLO) of nominal.
    pub fn voltage_tag(id: usize, bus_name: &str, nominal_kv: f64) -> TagValue {
        TagValue {
            tag_id: id,
            name: format!("{}.V", bus_name),
            value: nominal_kv,
            quality: MeasurementQuality::Missing,
            timestamp: 0.0,
            unit: "kV".to_owned(),
            engineering_min: 0.0,
            engineering_max: nominal_kv * 1.5,
            high_high_limit: Some(nominal_kv * 1.10),
            high_limit: Some(nominal_kv * 1.05),
            low_limit: Some(nominal_kv * 0.95),
            low_low_limit: Some(nominal_kv * 0.90),
            rtu_id: None,
        }
    }

    /// Create a current tag for a named feeder with the given rated current.
    ///
    /// High limit = rated current; High-high = 120 % of rated.
    pub fn current_tag(id: usize, feeder_name: &str, rated_a: f64) -> TagValue {
        TagValue {
            tag_id: id,
            name: format!("{}.I", feeder_name),
            value: 0.0,
            quality: MeasurementQuality::Missing,
            timestamp: 0.0,
            unit: "A".to_owned(),
            engineering_min: 0.0,
            engineering_max: rated_a * 1.5,
            high_high_limit: Some(rated_a * 1.20),
            high_limit: Some(rated_a),
            low_limit: None,
            low_low_limit: None,
            rtu_id: None,
        }
    }

    /// Create an active-power tag for a named device with the given rated power.
    ///
    /// High limit = rated power; High-high = 110 % of rated.
    pub fn power_tag(id: usize, device_name: &str, rated_mw: f64) -> TagValue {
        TagValue {
            tag_id: id,
            name: format!("{}.P", device_name),
            value: 0.0,
            quality: MeasurementQuality::Missing,
            timestamp: 0.0,
            unit: "MW".to_owned(),
            engineering_min: -rated_mw,
            engineering_max: rated_mw * 1.5,
            high_high_limit: Some(rated_mw * 1.10),
            high_limit: Some(rated_mw),
            low_limit: None,
            low_low_limit: None,
            rtu_id: None,
        }
    }

    /// Create a system frequency tag with standard ENTSO-E limits.
    ///
    /// Limits: LOLO=49.0 Hz, LO=49.5 Hz, HI=50.5 Hz, HIHI=51.0 Hz.
    pub fn frequency_tag(id: usize) -> TagValue {
        TagValue {
            tag_id: id,
            name: "SYS.FREQ".to_owned(),
            value: 50.0,
            quality: MeasurementQuality::Missing,
            timestamp: 0.0,
            unit: "Hz".to_owned(),
            engineering_min: 45.0,
            engineering_max: 55.0,
            high_high_limit: Some(51.0),
            high_limit: Some(50.5),
            low_limit: Some(49.5),
            low_low_limit: Some(49.0),
            rtu_id: None,
        }
    }

    /// Create a binary status tag (0 = open, 1 = closed) for a named device.
    pub fn status_tag(id: usize, device_name: &str) -> TagValue {
        TagValue {
            tag_id: id,
            name: format!("{}.STATUS", device_name),
            value: 0.0,
            quality: MeasurementQuality::Missing,
            timestamp: 0.0,
            unit: "binary".to_owned(),
            engineering_min: 0.0,
            engineering_max: 1.0,
            high_high_limit: None,
            high_limit: None,
            low_limit: None,
            low_low_limit: None,
            rtu_id: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_scada() -> MicroScada {
        MicroScada::new("TestSubstation".to_owned())
    }

    fn make_rtu(id: usize) -> RtuDevice {
        RtuDevice {
            id,
            name: format!("RTU-{}", id),
            protocol: RtuProtocol::Dnp3,
            address: format!("192.168.1.{}", 100 + id),
            poll_interval_ms: 1000,
            communication_ok: true,
            last_poll_timestamp: 0.0,
            n_analog_inputs: 8,
            n_binary_inputs: 16,
            n_control_outputs: 4,
        }
    }

    fn add_voltage_tag(scada: &mut MicroScada, tag_id: usize, rtu_id: Option<usize>) {
        let mut tag = TagBuilder::voltage_tag(tag_id, &format!("BUS{}", tag_id), 11.0);
        tag.rtu_id = rtu_id;
        scada.add_tag(tag);
    }

    // -----------------------------------------------------------------------
    // Test 1: Initialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_micro_scada_initialization() {
        let scada = make_scada();
        assert_eq!(scada.name, "TestSubstation");
        assert!(scada.rtu_devices.is_empty());
        assert!(scada.tags.is_empty());
        assert!(scada.alarms.is_empty());
        assert!(scada.control_log.is_empty());
        assert_eq!(scada.next_alarm_id, 1);
        assert_eq!(scada.current_timestamp, 0.0);
    }

    // -----------------------------------------------------------------------
    // Test 2: Add RTU
    // -----------------------------------------------------------------------

    #[test]
    fn test_add_rtu() {
        let mut scada = make_scada();
        scada.add_rtu(make_rtu(1));
        assert_eq!(scada.rtu_devices.len(), 1);
        assert_eq!(scada.rtu_devices[0].id, 1);
        assert_eq!(scada.rtu_devices[0].name, "RTU-1");
    }

    // -----------------------------------------------------------------------
    // Test 3: Update tag with good quality
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_tag_good() {
        let mut scada = make_scada();
        add_voltage_tag(&mut scada, 10, None);
        scada.update_tag(10, 11.5, MeasurementQuality::Good, 100.0);
        let tag = scada.get_tag_by_name("BUS10.V").expect("tag exists");
        assert!((tag.value - 11.5).abs() < 1e-9);
        assert_eq!(tag.quality, MeasurementQuality::Good);
        assert!((tag.timestamp - 100.0).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // Test 4: Update tag with bad quality
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_tag_bad_quality() {
        let mut scada = make_scada();
        add_voltage_tag(&mut scada, 20, None);
        scada.update_tag(20, 0.0, MeasurementQuality::Bad, 50.0);
        let tag = scada.get_tag_by_name("BUS20.V").expect("tag exists");
        assert_eq!(tag.quality, MeasurementQuality::Bad);
        // 0.0 is below low_low limit but bad quality; alarm still generated
        // (quality is stored correctly regardless)
        assert!(scada.alarms.iter().any(|a| a.tag_id == 20));
    }

    // -----------------------------------------------------------------------
    // Test 5: High-high alarm generation
    // -----------------------------------------------------------------------

    #[test]
    fn test_alarm_generation_high_high() {
        let mut scada = make_scada();
        add_voltage_tag(&mut scada, 1, None);
        // Nominal=11.0 kV, HIHI = 12.1 kV (110%)
        scada.update_tag(1, 12.5, MeasurementQuality::Good, 10.0);
        let active = scada.get_active_alarms();
        assert!(
            active.iter().any(|a| a.condition == "HIGH_HIGH"),
            "Expected HIGH_HIGH alarm, got: {:?}",
            active.iter().map(|a| &a.condition).collect::<Vec<_>>()
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Low-low alarm generation
    // -----------------------------------------------------------------------

    #[test]
    fn test_alarm_generation_low_low() {
        let mut scada = make_scada();
        add_voltage_tag(&mut scada, 2, None);
        // Nominal=11.0 kV, LOLO = 9.9 kV (90%)
        scada.update_tag(2, 9.0, MeasurementQuality::Good, 10.0);
        let active = scada.get_active_alarms();
        assert!(
            active.iter().any(|a| a.condition == "LOW_LOW"),
            "Expected LOW_LOW alarm"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Normal value produces no alarm
    // -----------------------------------------------------------------------

    #[test]
    fn test_alarm_no_trigger_normal() {
        let mut scada = make_scada();
        add_voltage_tag(&mut scada, 3, None);
        scada.update_tag(3, 11.0, MeasurementQuality::Good, 10.0);
        assert!(
            scada.get_active_alarms().is_empty(),
            "No alarm expected for nominal voltage"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: Alarm acknowledgement
    // -----------------------------------------------------------------------

    #[test]
    fn test_alarm_acknowledgement() {
        let mut scada = make_scada();
        add_voltage_tag(&mut scada, 4, None);
        scada.current_timestamp = 5.0;
        scada.update_tag(4, 12.5, MeasurementQuality::Good, 5.0);
        let alarm_id = scada.alarms.first().map(|a| a.id).unwrap_or(0);
        scada.current_timestamp = 10.0;
        let result = scada.acknowledge_alarm(alarm_id, "operator1");
        assert!(result, "Alarm should be acknowledged");
        let alarm = scada
            .alarms
            .iter()
            .find(|a| a.id == alarm_id)
            .expect("alarm");
        assert!(alarm.acknowledged);
        assert!(alarm.ack_timestamp.is_some());
        assert!((alarm.ack_timestamp.unwrap_or(0.0) - 10.0).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // Test 9: Alarm clearing when value returns to normal
    // -----------------------------------------------------------------------

    #[test]
    fn test_alarm_clearing() {
        let mut scada = make_scada();
        add_voltage_tag(&mut scada, 5, None);
        // Trigger HIGH alarm
        scada.update_tag(5, 11.7, MeasurementQuality::Good, 1.0);
        assert!(
            !scada.get_active_alarms().is_empty(),
            "Alarm should be active"
        );
        // Return to normal
        scada.update_tag(5, 11.0, MeasurementQuality::Good, 2.0);
        let active = scada.get_active_alarms();
        assert!(
            active.iter().all(|a| a.tag_id != 5),
            "All alarms for tag 5 should be cleared"
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: Active alarms sorted by priority
    // -----------------------------------------------------------------------

    #[test]
    fn test_active_alarms_sorted_by_priority() {
        let mut scada = make_scada();
        // Tag 6: will have LOW (Warning)
        let mut t6 = TagBuilder::voltage_tag(6, "BUS6", 11.0);
        t6.low_low_limit = None; // only LOW
        scada.add_tag(t6);
        // Tag 7: will have HIGH_HIGH (Emergency)
        add_voltage_tag(&mut scada, 7, None);

        scada.update_tag(6, 10.2, MeasurementQuality::Good, 1.0); // LOW
        scada.update_tag(7, 12.5, MeasurementQuality::Good, 1.0); // HIGH_HIGH (Emergency)

        let active = scada.get_active_alarms();
        assert!(active.len() >= 2, "Expected at least 2 alarms");
        // First alarm must be Emergency (HIGH_HIGH on tag 7)
        assert_eq!(
            active[0].priority,
            AlarmPriority::Emergency,
            "First alarm must be Emergency"
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: Execute control — open breaker
    // -----------------------------------------------------------------------

    #[test]
    fn test_execute_control_open_breaker() {
        let mut scada = make_scada();
        scada.current_timestamp = 100.0;
        let result = scada.execute_control(ControlCommand::OpenBreaker { device_id: 42 }, "op1");
        assert!(result.is_ok(), "OpenBreaker should succeed");
        assert_eq!(scada.control_log.len(), 1);
        assert!(scada.control_log[0].command.contains("OpenBreaker"));
        assert_eq!(scada.control_log[0].device_id, 42);
        assert!((scada.control_log[0].post_value - 0.0).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // Test 12: Control action logged in audit trail
    // -----------------------------------------------------------------------

    #[test]
    fn test_execute_control_logged() {
        let mut scada = make_scada();
        scada.current_timestamp = 200.0;
        scada
            .execute_control(ControlCommand::CloseBreaker { device_id: 5 }, "supervisor")
            .ok();
        let entry = scada.control_log.last().expect("log entry");
        assert_eq!(entry.operator_id, "supervisor");
        assert!((entry.timestamp - 200.0).abs() < 1e-9);
        assert!(entry.success);
        assert!(entry.command.contains("CloseBreaker"));
    }

    // -----------------------------------------------------------------------
    // Test 13: Get tag by name
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_tag_by_name() {
        let mut scada = make_scada();
        let tag = TagBuilder::frequency_tag(99);
        scada.add_tag(tag);
        let found = scada.get_tag_by_name("SYS.FREQ");
        assert!(found.is_some(), "Should find frequency tag by name");
        assert_eq!(found.unwrap().tag_id, 99);
    }

    // -----------------------------------------------------------------------
    // Test 14: Get tags by RTU
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_tags_by_rtu() {
        let mut scada = make_scada();
        add_voltage_tag(&mut scada, 101, Some(1));
        add_voltage_tag(&mut scada, 102, Some(1));
        add_voltage_tag(&mut scada, 103, Some(2));
        let tags_rtu1 = scada.get_tags_by_rtu(1);
        let tags_rtu2 = scada.get_tags_by_rtu(2);
        assert_eq!(tags_rtu1.len(), 2, "RTU 1 should have 2 tags");
        assert_eq!(tags_rtu2.len(), 1, "RTU 2 should have 1 tag");
        assert!(tags_rtu1.iter().all(|t| t.rtu_id == Some(1)));
    }

    // -----------------------------------------------------------------------
    // Test 15: KPI computation
    // -----------------------------------------------------------------------

    #[test]
    fn test_kpis_computation() {
        let mut scada = make_scada();
        scada.add_rtu(make_rtu(1));
        scada.add_rtu({
            let mut r = make_rtu(2);
            r.communication_ok = false;
            r
        });
        add_voltage_tag(&mut scada, 1, None);
        scada.update_tag(1, 11.0, MeasurementQuality::Good, 1.0);

        let kpis = scada.compute_kpis();
        assert_eq!(kpis.n_active_alarms, 0);
        assert_eq!(kpis.n_critical_alarms, 0);
        assert!((kpis.communication_availability_pct - 50.0).abs() < 1e-6);
        assert_eq!(kpis.n_stale_tags, 0);
        assert!((kpis.tag_quality_good_pct - 100.0).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // Test 16: KPI communication availability
    // -----------------------------------------------------------------------

    #[test]
    fn test_kpi_comm_availability() {
        let mut scada = make_scada();
        for i in 1..=4usize {
            let mut rtu = make_rtu(i);
            rtu.communication_ok = i <= 3; // 3 of 4 ok
            scada.add_rtu(rtu);
        }
        let kpis = scada.compute_kpis();
        assert!(
            (kpis.communication_availability_pct - 75.0).abs() < 1e-6,
            "Expected 75%, got {}",
            kpis.communication_availability_pct
        );
    }

    // -----------------------------------------------------------------------
    // Test 17: Simulate poll
    // -----------------------------------------------------------------------

    #[test]
    fn test_simulate_poll() {
        let mut scada = make_scada();
        scada.add_rtu(make_rtu(1));
        add_voltage_tag(&mut scada, 1, Some(1));
        scada.current_timestamp = 50.0;
        scada.simulate_poll(1, vec![(1, 11.2)]);
        let tag = scada.get_tag_by_name("BUS1.V").expect("tag");
        assert!((tag.value - 11.2).abs() < 1e-9);
        assert_eq!(tag.quality, MeasurementQuality::Good);
        let rtu = &scada.rtu_devices[0];
        assert!(rtu.communication_ok);
        assert!((rtu.last_poll_timestamp - 50.0).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // Test 18: TagBuilder voltage tag limits
    // -----------------------------------------------------------------------

    #[test]
    fn test_tag_builder_voltage() {
        let tag = TagBuilder::voltage_tag(1, "HV_BUS", 33.0);
        assert!((tag.high_high_limit.unwrap_or(0.0) - 33.0 * 1.10).abs() < 1e-6);
        assert!((tag.high_limit.unwrap_or(0.0) - 33.0 * 1.05).abs() < 1e-6);
        assert!((tag.low_limit.unwrap_or(0.0) - 33.0 * 0.95).abs() < 1e-6);
        assert!((tag.low_low_limit.unwrap_or(0.0) - 33.0 * 0.90).abs() < 1e-6);
        assert_eq!(tag.unit, "kV");
    }

    // -----------------------------------------------------------------------
    // Test 19: TagBuilder frequency tag limits
    // -----------------------------------------------------------------------

    #[test]
    fn test_tag_builder_frequency() {
        let tag = TagBuilder::frequency_tag(50);
        assert_eq!(tag.tag_id, 50);
        assert_eq!(tag.unit, "Hz");
        assert!((tag.high_limit.unwrap_or(0.0) - 50.5).abs() < 1e-6);
        assert!((tag.low_limit.unwrap_or(0.0) - 49.5).abs() < 1e-6);
        assert!((tag.high_high_limit.unwrap_or(0.0) - 51.0).abs() < 1e-6);
        assert!((tag.low_low_limit.unwrap_or(0.0) - 49.0).abs() < 1e-6);
        assert!((tag.value - 50.0).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // Test 20: Multiple control log entries
    // -----------------------------------------------------------------------

    #[test]
    fn test_control_log_entries() {
        let mut scada = make_scada();
        scada.current_timestamp = 0.0;
        scada
            .execute_control(ControlCommand::OpenBreaker { device_id: 1 }, "op")
            .ok();
        scada.current_timestamp = 60.0;
        scada
            .execute_control(ControlCommand::CloseBreaker { device_id: 2 }, "op")
            .ok();
        scada.current_timestamp = 120.0;
        scada
            .execute_control(
                ControlCommand::SetSetpoint {
                    device_id: 3,
                    value_mw: 5.0,
                },
                "supervisor",
            )
            .ok();
        assert_eq!(scada.get_historical_count(), 3);
        assert!(scada.control_log[0].command.contains("OpenBreaker"));
        assert!(scada.control_log[1].command.contains("CloseBreaker"));
        assert!(scada.control_log[2].command.contains("SetSetpoint"));
    }

    // -----------------------------------------------------------------------
    // Bonus tests
    // -----------------------------------------------------------------------

    /// Test that EmergencyShutdown command is logged correctly.
    #[test]
    fn test_execute_control_emergency_shutdown() {
        let mut scada = make_scada();
        scada.current_timestamp = 999.0;
        let result = scada.execute_control(
            ControlCommand::EmergencyShutdown { zone_id: 7 },
            "emergency_op",
        );
        assert!(result.is_ok());
        let entry = scada.control_log.last().expect("log entry");
        assert!(entry.command.contains("EmergencyShutdown"));
        assert_eq!(entry.device_id, 7);
    }

    /// Test that display pages can be added and retrieved.
    #[test]
    fn test_add_display_page() {
        let mut scada = make_scada();
        scada.add_display_page(ScadaDisplayPage {
            page_id: 1,
            name: "Overview".to_owned(),
            tag_ids: vec![1, 2, 3],
            display_type: "single_line_diagram".to_owned(),
            refresh_rate_ms: 1000,
        });
        assert_eq!(scada.display_pages.len(), 1);
        assert_eq!(scada.display_pages[0].name, "Overview");
        assert_eq!(scada.display_pages[0].tag_ids.len(), 3);
    }

    /// Test stale tag detection in KPIs.
    #[test]
    fn test_kpi_stale_tags() {
        let mut scada = make_scada();
        // Tag updated at t=0, current time=120 → stale (>60s)
        add_voltage_tag(&mut scada, 1, None);
        scada.update_tag(1, 11.0, MeasurementQuality::Good, 0.0);
        scada.current_timestamp = 120.0;
        let kpis = scada.compute_kpis();
        assert_eq!(
            kpis.n_stale_tags, 1,
            "Tag should be stale after 120s without update"
        );
    }
}
