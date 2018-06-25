// Copyright (c) 2017-2018 Rene van der Meer
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! Raspberry Pi system-related tools.
//!
//! Use [`DeviceInfo`] to identify what Raspberry Pi model and SoC the software is
//! running on. This information is used internally to calculate the correct memory
//! locations for the various BCM283x peripherals.
//!
//! [`DeviceInfo`]: struct.DeviceInfo.html

use std::fmt;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::result;

const PERIPHERAL_BASE_RPI: u32 = 0x2000_0000;
const PERIPHERAL_BASE_RPI2: u32 = 0x3f00_0000;
const GPIO_OFFSET: u32 = 0x20_0000;

quick_error! {
/// Errors that can occur when trying to identify the Raspberry Pi hardware.
    #[derive(Debug)]
    pub enum Error {
/// Unknown model.
///
/// `DeviceInfo` was unable to identify the Raspberry Pi model based on the
/// contents of `/proc/cpuinfo`, `/sys/firmware/devicetree/base/compatible`
/// and `/sys/firmware/devicetree/base/model`.
        UnknownModel { description("unknown Raspberry Pi model") }
    }
}

/// Result type returned from methods that can have `system::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// Identifiable Raspberry Pi models.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Model {
    RaspberryPiA,
    RaspberryPiAPlus,
    RaspberryPiBRev1,
    RaspberryPiBRev2,
    RaspberryPiBPlus,
    RaspberryPi2B,
    RaspberryPi3B,
    RaspberryPi3BPlus,
    RaspberryPiComputeModule,
    RaspberryPiComputeModule3,
    RaspberryPiZero,
    RaspberryPiZeroW,
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Model::RaspberryPiA => write!(f, "Raspberry Pi A"),
            Model::RaspberryPiAPlus => write!(f, "Raspberry Pi A+"),
            Model::RaspberryPiBRev1 => write!(f, "Raspberry Pi B Rev 1"),
            Model::RaspberryPiBRev2 => write!(f, "Raspberry Pi B Rev 2"),
            Model::RaspberryPiBPlus => write!(f, "Raspberry Pi B+"),
            Model::RaspberryPi2B => write!(f, "Raspberry Pi 2 B"),
            Model::RaspberryPi3B => write!(f, "Raspberry Pi 3 B"),
            Model::RaspberryPi3BPlus => write!(f, "Raspberry Pi 3 B+"),
            Model::RaspberryPiComputeModule => write!(f, "Raspberry Pi Compute Module"),
            Model::RaspberryPiComputeModule3 => write!(f, "Raspberry Pi Compute Module 3"),
            Model::RaspberryPiZero => write!(f, "Raspberry Pi Zero"),
            Model::RaspberryPiZeroW => write!(f, "Raspberry Pi Zero W"),
        }
    }
}

/// Identifiable Raspberry Pi SoCs.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SoC {
    Bcm2835,
    Bcm2836,
    Bcm2837A1,
    Bcm2837B0,
}

impl fmt::Display for SoC {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SoC::Bcm2835 => write!(f, "BCM2835"),
            SoC::Bcm2836 => write!(f, "BCM2836"),
            SoC::Bcm2837A1 => write!(f, "BCM2837A1"),
            SoC::Bcm2837B0 => write!(f, "BCM2837B0"),
        }
    }
}

// Identify Pi model based on /proc/cpuinfo
fn parse_proc_cpuinfo() -> Result<Model> {
    let proc_cpuinfo = BufReader::new(match File::open("/proc/cpuinfo") {
        Err(_) => return Err(Error::UnknownModel),
        Ok(file) => file,
    });

    let mut hardware: String = String::new();
    let mut revision: String = String::new();
    for line_result in proc_cpuinfo.lines() {
        if let Ok(line) = line_result {
            if line.starts_with("Hardware\t: ") {
                hardware = String::from(&line[11..]);
            } else if line.starts_with("Revision\t: ") {
                revision = String::from(&line[11..]).to_lowercase();
            }
        }
    }

    // Return an error if we don't recognize the SoC. This check is
    // done to prevent accidentally identifying a non-Pi SBC as a Pi
    // solely based on the revision field.
    match &hardware[..] {
        "BCM2708" | "BCM2835" | "BCM2709" | "BCM2836" | "BCM2710" | "BCM2837" | "BCM2837A1"
        | "BCM2837B0" => {}
        _ => return Err(Error::UnknownModel),
    }

    let model = if (revision.len() == 4) || (revision.len() == 8) {
        // Older revisions are 4 characters long, or 8 if they've been over-volted
        match &revision[revision.len() - 4..] {
            "0007" | "0008" | "0009" | "0015" => Model::RaspberryPiA,
            "Beta" | "0002" | "0003" => Model::RaspberryPiBRev1,
            "0004" | "0005" | "0006" | "000d" | "000e" | "000f" => Model::RaspberryPiBRev2,
            "0012" => Model::RaspberryPiAPlus,
            "0010" | "0013" => Model::RaspberryPiBPlus,
            "0011" | "0014" => Model::RaspberryPiComputeModule,
            _ => return Err(Error::UnknownModel),
        }
    } else if revision.len() >= 6 {
        // Newer revisions consist of at least 6 characters
        match &revision[revision.len() - 3..revision.len() - 1] {
            "00" => Model::RaspberryPiA,
            "01" => Model::RaspberryPiBRev2,
            "02" => Model::RaspberryPiAPlus,
            "03" => Model::RaspberryPiBPlus,
            "04" => Model::RaspberryPi2B,
            "06" => Model::RaspberryPiComputeModule,
            "08" => Model::RaspberryPi3B,
            "09" => Model::RaspberryPiZero,
            "0a" => Model::RaspberryPiComputeModule3,
            "0c" => Model::RaspberryPiZeroW,
            "0d" => Model::RaspberryPi3BPlus,
            _ => return Err(Error::UnknownModel),
        }
    } else {
        return Err(Error::UnknownModel);
    };

    Ok(model)
}

// Identify Pi model based on /sys/firmware/devicetree/base/compatible
fn parse_firmware_compatible() -> Result<Model> {
    let compatible = match fs::read_to_string("/sys/firmware/devicetree/base/compatible") {
        Err(_) => return Err(Error::UnknownModel),
        Ok(s) => s,
    };

    // Based on /arch/arm/boot/dts/ and /Documentation/devicetree/bindings/arm/bcm/
    for compid in compatible.split('\0') {
        match compid {
            "raspberrypi,model-b-i2c0" => return Ok(Model::RaspberryPiBRev1),
            "raspberrypi,model-b" => return Ok(Model::RaspberryPiBRev1),
            "raspberrypi,model-a" => return Ok(Model::RaspberryPiA),
            "raspberrypi,model-b-rev2" => return Ok(Model::RaspberryPiBRev2),
            "raspberrypi,model-a-plus" => return Ok(Model::RaspberryPiAPlus),
            "raspberrypi,model-b-plus" => return Ok(Model::RaspberryPiBPlus),
            "raspberrypi,2-model-b" => return Ok(Model::RaspberryPi2B),
            "raspberrypi,compute-module" => return Ok(Model::RaspberryPiComputeModule),
            "raspberrypi,3-model-b" => return Ok(Model::RaspberryPi3B),
            "raspberrypi,model-zero" => return Ok(Model::RaspberryPiZero),
            "raspberrypi,3-compute-module" => return Ok(Model::RaspberryPiComputeModule3),
            "raspberrypi,model-zero-w" => return Ok(Model::RaspberryPiZeroW),
            "raspberrypi,3-model-b-plus" => return Ok(Model::RaspberryPi3BPlus),
            _ => (),
        }
    }

    Err(Error::UnknownModel)
}

// Identify Pi model based on /sys/firmware/devicetree/base/model
fn parse_firmware_model() -> Result<Model> {
    let model = match fs::read_to_string("/sys/firmware/devicetree/base/model") {
        Err(_) => return Err(Error::UnknownModel),
        Ok(buffer) => if let Some(rev_idx) = buffer.find(" Rev ") {
            // We don't want to strip rev2 here
            buffer[0..rev_idx].to_owned()
        } else {
            buffer
        },
    };

    // Based on /arch/arm/boot/dts/ and /Documentation/devicetree/bindings/arm/bcm/
    match &model[..] {
        "Raspberry Pi Model B (no P5)" => Ok(Model::RaspberryPiBRev1),
        "Raspberry Pi Model B" => Ok(Model::RaspberryPiBRev1),
        "Raspberry Pi Model A" => Ok(Model::RaspberryPiA),
        "Raspberry Pi Model B rev2" => Ok(Model::RaspberryPiBRev2),
        "Raspberry Pi Model A+" => Ok(Model::RaspberryPiAPlus),
        "Raspberry Pi Model B+" => Ok(Model::RaspberryPiBPlus),
        "Raspberry Pi 2 Model B" => Ok(Model::RaspberryPi2B),
        "Raspberry Pi Compute Module" => Ok(Model::RaspberryPiComputeModule),
        "Raspberry Pi 3 Model B" => Ok(Model::RaspberryPi3B),
        "Raspberry Pi Zero" => Ok(Model::RaspberryPiZero),
        "Raspberry Pi Compute Module 3" => Ok(Model::RaspberryPiComputeModule3),
        "Raspberry Pi Zero W" => Ok(Model::RaspberryPiZeroW),
        "Raspberry Pi 3 Model B+" => Ok(Model::RaspberryPi3BPlus),
        _ => Err(Error::UnknownModel),
    }
}

/// Retrieves Raspberry Pi device information.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct DeviceInfo {
    model: Model,
    soc: SoC,
    peripheral_base: u32,
    gpio_offset: u32,
}

impl DeviceInfo {
    /// Constructs a new `DeviceInfo`.
    ///
    /// `new` parses the contents of `/proc/cpuinfo` to identify the Raspberry
    /// Pi's model and SoC.
    pub fn new() -> Result<DeviceInfo> {
        let model = parse_proc_cpuinfo()
            .or_else(|_| parse_firmware_compatible().or_else(|_| parse_firmware_model()))?;

        // Set SoC and memory offsets based on model
        match model {
            Model::RaspberryPiA
            | Model::RaspberryPiAPlus
            | Model::RaspberryPiBRev1
            | Model::RaspberryPiBRev2
            | Model::RaspberryPiBPlus
            | Model::RaspberryPiComputeModule
            | Model::RaspberryPiZero
            | Model::RaspberryPiZeroW => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2835,
                peripheral_base: PERIPHERAL_BASE_RPI,
                gpio_offset: GPIO_OFFSET,
            }),
            Model::RaspberryPi2B => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2836,
                peripheral_base: PERIPHERAL_BASE_RPI2,
                gpio_offset: GPIO_OFFSET,
            }),
            Model::RaspberryPi3B | Model::RaspberryPiComputeModule3 => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2837A1,
                peripheral_base: PERIPHERAL_BASE_RPI2,
                gpio_offset: GPIO_OFFSET,
            }),
            Model::RaspberryPi3BPlus => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2837B0,
                peripheral_base: PERIPHERAL_BASE_RPI2,
                gpio_offset: GPIO_OFFSET,
            }),
        }
    }

    /// Returns the Raspberry Pi's model.
    pub fn model(&self) -> Model {
        self.model
    }

    /// Returns the Raspberry Pi's SoC.
    pub fn soc(&self) -> SoC {
        self.soc
    }

    /// Returns the base memory address for the BCM283x peripherals.
    pub fn peripheral_base(&self) -> u32 {
        self.peripheral_base
    }

    /// Returns the offset from the base memory address for the GPIO section.
    pub fn gpio_offset(&self) -> u32 {
        self.gpio_offset
    }
}
