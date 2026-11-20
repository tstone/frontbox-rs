/// Which kind a driver is: high voltage for coils, flashers and motors, or low voltage for lamps and other
/// small loads. It comes from the board a driver is wired to ([`IoBoard::voltage_at`]).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DriverVoltage {
  High,
  Low,
}
