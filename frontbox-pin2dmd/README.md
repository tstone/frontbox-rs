## DMD Hardware Setup

### Windows

The PIN2DMD requires the **libusb-win32** driver to be installed before it can communicate with software using libusb.

1. Download and run **[Zadig](https://zadig.akeo.ie/)**
2. Plug in your PIN2DMD via USB
3. If the device doesn't appear in Zadig, go to **Options → List All Devices**
4. Select the **PIN2DMD** device (VID `0314`, PID `E457`)
5. Select **libusb-win32** as the driver
6. Click **Replace Driver**

> If you need to restore the original driver (e.g. to use the PIN2DMD's own configuration software), you can do so from Device Manager.

### Linux

No driver installation is needed on Linux, but by default USB devices are only accessible as root. To allow access as a normal user, add a udev rule:

1. Create `/etc/udev/rules.d/99-pin2dmd.rules` with the following content:
```
   SUBSYSTEM=="usb", ATTRS{idVendor}=="0314", ATTRS{idProduct}=="e457", MODE="0666"
```

2. Reload udev rules:
```bash
   sudo udevadm control --reload-rules
   sudo udevadm trigger
```

3. Unplug and replug the PIN2DMD

You can verify the device is visible with `lsusb` — look for VID `0314` PID `e457`.
