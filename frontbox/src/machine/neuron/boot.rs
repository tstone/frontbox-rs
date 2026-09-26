use std::time::Duration;

use fast_protocol::net::prelude::*;

use crate::prelude::serial_interface::SerialInterface;
use crate::prelude::*;

/// wait for the mainboard to be ready to respond
pub async fn run(io_port: &mut SerialInterface) -> String {
  loop {
    match io_port
      .request(&IdCommand::new(), Duration::from_millis(500))
      .await
    {
      Ok(IdResponse::Report { mainboard_name, .. }) => {
        // Initialize the mainboard with the right firmware
        let _ = io_port
          .request(
            &ConfigureHardwareCommand::new(
              FastPlatform::Neuron.into(),
              Some(SwitchReporting::Verbose),
            ),
            Duration::from_millis(500),
          )
          .await;

        log::info!("🥾 Mainboard is ready");
        return mainboard_name;
      }
      _ => {
        log::debug!("Mainboard not ready, retrying...");
        tokio::time::sleep(Duration::from_millis(500)).await;
      }
    };
  }
}

/// Verify the watchdog is responsive. Sometimes the first few commands will fail.
pub async fn verify_watchdog(io_port: &mut SerialInterface) {
  loop {
    match io_port
      .request(
        &WatchdogCommand::set(Duration::from_millis(1250)),
        Duration::from_secs(1),
      )
      .await
    {
      Ok(WatchdogResponse::Processed) => {
        log::info!("🥾 Watchdog is ready");
        break;
      }
      _ => {
        tokio::time::sleep(Duration::from_millis(500)).await;
      }
    };
  }
}

pub async fn configure_switches(
  io_port: &mut SerialInterface,
  switches: &Vec<IoAddressed<SwitchDefinition>>,
) {
  for switch in switches {
    if let Some(config) = &switch.definition.config {
      let reporting = if config.inverted {
        SwitchReportingMode::ReportInverted
      } else {
        SwitchReportingMode::ReportNormal
      };
      log::info!(
        "Configuring switch {} with {:?}",
        switch.definition.name,
        config
      );
      let _ = io_port
        .request(
          &ConfigureSwitchCommand::new(
            switch.id,
            reporting,
            config.debounce_close,
            config.debounce_open,
          ),
          Duration::from_millis(500),
        )
        .await;
    }
  }
}
