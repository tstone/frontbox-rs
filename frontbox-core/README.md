# frontbox-core

## Neuron Hardware Setup

The Neuron connects over USB as three serial ports. Frontbox uses the first two: the I/O network
(`io_net_port_path`) and the expansion network (`exp_port_path`).

### Linux

Linux numbers USB serial ports (`/dev/ttyACM0`, `/dev/ttyACM1`, ...) in the order devices appear. After a reboot,
a replug, or a USB hub reset, the Neuron can come back under different numbers. Point Frontbox at the
`/dev/serial/by-id` names instead. They're built from the device's serial number and USB interface, so they always
lead to the same port.

1. With the Neuron plugged in, list its ports:

```bash
   ls -l /dev/serial/by-id/
```

```
   usb-FAST_Pinball_Neuron_Controller_E663A0079B4DB637-if00 -> ../../ttyACM0
   usb-FAST_Pinball_Neuron_Controller_E663A0079B4DB637-if02 -> ../../ttyACM1
   usb-FAST_Pinball_Neuron_Controller_E663A0079B4DB637-if04 -> ../../ttyACM2
```

2. Use `-if00` for the I/O network and `-if02` for the expansion network. The serial number is your board's own.

```rust
   Platform::Neuron {
     io_net_port_path: "/dev/serial/by-id/usb-FAST_Pinball_Neuron_Controller_E663A0079B4DB637-if00",
     exp_port_path: "/dev/serial/by-id/usb-FAST_Pinball_Neuron_Controller_E663A0079B4DB637-if02",
     watchdog_interval: Duration::from_millis(1500),
   }
```

The ports belong to the `dialout` group, so the user the game runs as needs to be in it:

```bash
   sudo usermod -aG dialout $USER
```

(log out and back in for it to take effect)

#### When the Neuron isn't found

- If a port won't open, Frontbox logs the serial ports that exist and where each `/dev/serial/by-id` name points.
- `dmesg | tail -40`, run right after plugging the Neuron in, shows whether it was recognized. Each port appears as a
  `cdc_acm ... ttyACMn: USB ACM device` line.
- `device descriptor read/64, error -32` means the Neuron didn't connect cleanly over USB. That usually comes from a
  hub, through power or signal quality. Plug it straight into the computer or use a powered hub.

### Windows

Windows gives each port a COM number (`COM3`, `COM4`, ...) and keeps it for that device. Find them in Device Manager
under **Ports (COM & LPT)**. To tell them apart, open a port's **Properties → Details → Hardware Ids**: `MI_00` is
the I/O network and `MI_02` the expansion network, the same interfaces as `-if00` and `-if02` on Linux.
