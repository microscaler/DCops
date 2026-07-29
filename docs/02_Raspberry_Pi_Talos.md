# Raspberry Pi + Talos Linux Considerations

## Overview

DCops manages Raspberry Pi compute blades running Talos Linux. This document captures Raspberry Pi-specific installation and configuration requirements based on [Talos Linux Raspberry Pi documentation](https://docs.siderolabs.com/talos/v1.12/platform-specific-installations/single-board-computers/rpi_generic).

## Supported Hardware

**Officially Tested:**
- Raspberry Pi 4
- Raspberry Pi Compute Module 4 (with Super 6C boards)

**Community Tested:**
- Other boards supported by u-boot `rpi_arm64_defconfig`

**Note:** If testing on other Raspberry Pi variants, community feedback is valuable.

## Installation Methods

### 1. PXE Boot (DCops Primary Method)

DCops PXE Intent Controller will configure PXE boot for Raspberry Pi nodes:

- **Boot Method:** Network boot via PXE/iPXE
- **Image Source:** Talos installer image served via HTTP/TFTP
- **Configuration:** BootProfile CRD defines installer image and kernel parameters
- **Workflow:** MAC address → BootIntent → PXE service → Talos installer → Talos installation

### 2. SD Card Installation (Manual/Initial Setup)

For initial setup or recovery:

- **Image:** `metal-arm64.raw` disk image
- **Writing:** `dd` to SD card
- **Bootstrap:** Interactive installer via `talosctl apply-config --mode=interactive`

## Prerequisites

### EEPROM Update

**One-time requirement:** Update Raspberry Pi bootloader EEPROM before first use.

**Process:**
1. Use Raspberry Pi Imager to write EEPROM update image to SD card
2. Boot Raspberry Pi with update SD card
3. Wait for green LED rapid blink (success) or error pattern
4. Power off and remove SD card

**Note:** Only needs to be done once per board.

### Tools Required

- `talosctl` — Talos management CLI
- SD card (for initial setup or recovery)
- Network boot capability (for PXE workflow)

## Talos Image Configuration

### Default Image

**Schematic ID:** `ee21ef4a5ef808a9b7484cc0dda0f25075021691c8c09a276591eedb638ea1f9`

This is the "vanilla" Raspberry Pi generic image without special extensions.

### Custom Images via Image Factory

For Raspberry Pi-specific requirements (GPU support, custom config.txt), use Talos Image Factory:

**Image Factory URL Pattern:**
```
https://factory.talos.dev/image/{schematic_id}/{version}/metal-arm64.raw.xz
```

**Schematic Definition Example:**
```yaml
overlay:
  name: rpi_generic
  image: siderolabs/sbc-raspberrypi
  options:
    configTxt: |
      gpu_mem=128
      kernel=u-boot.bin
      arm_64bit=1
      arm_boost=1
      enable_uart=1
      dtoverlay=disable-bt
      dtoverlay=disable-wifi
      avoid_warnings=2
      dtoverlay=vc4-kms-v3d,noaudio
customization:
  systemExtensions:
    officialExtensions:
      - siderolabs/vc4
```

**DCops Integration:**
- BootProfile CRD can reference custom Image Factory schematic IDs
- PXE Intent Controller serves appropriate installer image based on hardware profile
- Supports different images for different Pi models or use cases

## GPU and Memory Considerations

### GPU Support (vc4 System Extension)

**Use Case:** Enable Broadcom VideoCore GPU support for V3D/VC4 operations.

**Requirements:**
- `vc4` system extension
- Custom `config.txt` configuration
- Sufficient CMA (Contiguous Memory Allocator) size

**CMA Size Guide:**

| CMA Size | Suitable For                  |
|----------|-------------------------------|
| 64 MB    | Headless, no GPU use          |
| 128 MB   | Light use                     |
| 256 MB   | HD media, cameras             |
| 512 MB   | 4K media, ML with GPU         |
| 1024 MB  | Experimental, may destabilize |

**Configuration:**
- Via kernel parameters: `cma=256M` in machine config
- Via Image Factory: `dtoverlay=vc4-kms-v3d,cma-256` in config.txt

**DCops Consideration:** BootProfile CRD should support CMA size configuration for GPU-enabled workloads.

## Boot Configuration (config.txt)

### Key Settings

**Default config.txt** (from `sbc-raspberrypi` overlay):
- `kernel=u-boot.bin` — Use u-boot bootloader
- `arm_64bit=1` — 64-bit mode
- `arm_boost=1` — Performance boost
- `enable_uart=1` — Serial console

**For GPU Support:**
- `gpu_mem=128` — GPU memory allocation
- `avoid_warnings=2` — Suppress warnings
- `dtoverlay=vc4-kms-v3d,noaudio` — Enable GPU overlay

**For Fan Control:**
- `dtoverlay=gpio-fan,gpiopin=14` — GPIO fan (default GPIO 12)
- `dtoverlay=pwm-gpio-fan` — PWM fan (default GPIO 18)

**Note:** GPIO 14 conflicts with UART. Use GPIO 4 or disable UART if using GPIO 14.

### DCops Integration

**BootProfile CRD** should support:
- Image Factory schematic ID selection
- Custom config.txt parameters
- System extension selection
- Kernel parameter overrides

## Installation Workflow

### PXE Boot Workflow (DCops)

```
1. Raspberry Pi powers on
   ↓
2. PXE boot initiated (via network)
   ↓
3. DCops PXE Intent Controller
   - Detects MAC address
   - Looks up BootIntent CRD
   - Configures PXE service with Talos installer
   ↓
4. PXE service serves Talos installer
   - Downloads installer image
   - Boots into installer
   ↓
5. Talos installer runs
   - Installs Talos to disk
   - Reboots into Talos
   ↓
6. CAPI Bootstrap Provider (CABPT)
   - Generates Talos machine config
   - Applies via Talos API
   ↓
7. Node joins Kubernetes cluster
```

### Interactive Installation (Manual)

For manual setup or recovery:

```bash
# 1. Write image to SD card
sudo dd if=metal-arm64.raw of=/dev/mmcblk0 conv=fsync bs=4M

# 2. Boot Raspberry Pi
# Wait for console instructions

# 3. Apply interactive config
talosctl apply-config \
  --insecure \
  --mode=interactive \
  --nodes <node-ip>

# 4. Retrieve kubeconfig
talosctl kubeconfig
```

## Upgrading Talos on Raspberry Pi

### Standard Upgrade

```bash
talosctl upgrade --image factory.talos.dev/installer/{schematic_id}:{version}
```

### Upgrade with New System Extensions

1. Generate new schematic with updated extensions
2. Get new schematic ID from Image Factory
3. Upgrade using new schematic ID:

```bash
talosctl upgrade --image factory.talos.dev/installer/{new_schematic_id}:{version}
```

**DCops Consideration:** Upgrade orchestration should handle schematic ID changes and system extension updates.

## Troubleshooting

### Boot Status LEDs

| Long Flashes | Short Flashes | Status                              |
|--------------|---------------|-------------------------------------|
| 0            | 3             | Generic failure to boot             |
| 0            | 4             | start\*.elf not found               |
| 0            | 7             | Kernel image not found              |
| 0            | 8             | SDRAM failure                       |
| 0            | 9             | Insufficient SDRAM                  |
| 0            | 10            | In HALT state                       |
| 2            | 1             | Partition not FAT                   |
| 2            | 2             | Failed to read from partition       |
| 2            | 3             | Extended partition not FAT          |
| 2            | 4             | File signature/hash mismatch - Pi 4 |
| 4            | 4             | Unsupported board type              |
| 4            | 5             | Fatal firmware error                |
| 4            | 6             | Power failure type A                |
| 4            | 7             | Power failure type B                |

### GPU Memory Issues

**Error:** `DRM_IOCTL_MODE_CREATE_DUMB failed: Cannot allocate memory`

**Cause:** Insufficient CMA size for GPU operations.

**Solution:** Increase CMA size via kernel parameters or Image Factory config.txt.

### HDMI Display Issues

**Issue:** Rainbow splash screen only.

**Solution:** Use HDMI port closest to power/USB-C port.

## DCops Integration Requirements

### BootProfile CRD Enhancements

Should support:
- **Image Factory schematic ID** — For custom Talos images
- **System extensions** — e.g., `vc4` for GPU support
- **config.txt parameters** — Raspberry Pi-specific boot config
- **CMA size** — For GPU-enabled workloads
- **Kernel parameters** — e.g., `cma=256M`

### PXE Intent Controller

Must handle:
- **Raspberry Pi PXE boot** — Network boot workflow
- **Installer image selection** — Based on hardware profile
- **MAC address mapping** — To BootIntent CRD
- **Boot configuration** — Talos installer parameters

### IP Claim Controller

Must handle:
- **Raspberry Pi network interfaces** — Typically single Ethernet interface
- **Static IP allocation** — For Talos API access
- **NetBox integration** — Device inventory with MAC addresses

## References

- [Talos Linux Raspberry Pi Documentation](https://docs.siderolabs.com/talos/v1.12/platform-specific-installations/single-board-computers/rpi_generic)
- [Talos Image Factory](https://www.talos.dev/latest/learn-more/image-factory/)
- [Talos System Extensions](https://www.talos.dev/latest/learn-more/system-extensions/)
- [Raspberry Pi Official Documentation](https://www.raspberrypi.com/documentation/)

