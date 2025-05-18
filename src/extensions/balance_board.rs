use crate::calibration::remap;
use crate::device::WiimoteDevice;
use crate::output::Addressing;
use crate::result::{WiimoteDeviceError, WiimoteResult};
use crate::simple_io;

#[derive(Debug, Default, Clone)]
pub struct BalanceBoardCalibration {
    weights_0kg: WeightData,
    weights_17kg: WeightData,
    weights_34kg: WeightData,
    reference_battery: u8,
    reference_temperature: u8,
}

impl BalanceBoardCalibration {
    pub(crate) fn read(wiimote: &WiimoteDevice) -> WiimoteResult<Self> {
        // https://www.wiibrew.org/wiki/Wii_Balance_Board#Calibration_Data
        let data_first_half =
            simple_io::read_16_bytes_sync(wiimote, Addressing::control_registers(0xA4_0020, 16))?;
        let data_second_half =
            simple_io::read_16_bytes_sync(wiimote, Addressing::control_registers(0xA4_0030, 16))?;

        let data_reference_temperature =
            simple_io::read_16_bytes_sync(wiimote, Addressing::control_registers(0xA4_0060, 2))?;

        let mut calibration_data = [0u8; 32];
        calibration_data[..16].copy_from_slice(&data_first_half.data);
        calibration_data[16..].copy_from_slice(&data_second_half.data);

        let mut checksum_data = [0u8; 4];
        checksum_data.copy_from_slice(&data_second_half.data[12..]);
        let checksum = u32::from_be_bytes(checksum_data);

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&calibration_data[4..28]); // 0x24..0x3B
        hasher.update(&calibration_data[..2]); // 0x20..0x21
        hasher.update(&data_reference_temperature.data[..2]); // 0x60..0x61

        if hasher.finalize() != checksum {
            return Err(WiimoteDeviceError::InvalidChecksum.into());
        }

        Ok(Self {
            weights_0kg: WeightData::read(&calibration_data[4..12])
                .ok_or(WiimoteDeviceError::InvalidData)?,
            weights_17kg: WeightData::read(&calibration_data[12..20])
                .ok_or(WiimoteDeviceError::InvalidData)?,
            weights_34kg: WeightData::read(&calibration_data[20..28])
                .ok_or(WiimoteDeviceError::InvalidData)?,
            reference_battery: calibration_data[1],
            reference_temperature: data_reference_temperature.data[0],
        })
    }

    /// Converts the weight data from the balance board to kg per area using the calibration.
    #[must_use]
    pub fn get_weights(&self, data: &BalanceBoardData) -> WeightValues {
        macro_rules! weight_value {
            ($position:ident) => {
                Self::get_weight_value(
                    data.weights.$position,
                    self.weights_0kg.$position,
                    self.weights_17kg.$position,
                    self.weights_34kg.$position,
                )
            };
        }

        let temperature_scale = data.temperature.map_or(1.0, |temperature| {
            let temp = temperature as f32 - self.reference_temperature as f32;
            0.999 * 0.0007f32.mul_add(-temp, 1.0)
        });

        WeightValues {
            top_right: weight_value!(top_right) * temperature_scale,
            bottom_right: weight_value!(bottom_right) * temperature_scale,
            top_left: weight_value!(top_left) * temperature_scale,
            bottom_left: weight_value!(bottom_left) * temperature_scale,
            battery: data
                .battery
                .map(|battery| battery.saturating_sub(self.reference_battery)),
        }
    }

    fn get_weight_value(value: u16, ref_0kg: u16, ref_17kg: u16, ref_34kg: u16) -> f32 {
        let value = value as f32;
        let ref_0kg = ref_0kg as f32;
        let ref_17kg = ref_17kg as f32;
        let ref_34kg = ref_34kg as f32;

        if value <= ref_0kg {
            0.0
        } else if value <= ref_17kg {
            remap(value, ref_0kg, ref_17kg, 0.0, 17.0)
        } else {
            remap(value, ref_17kg, ref_34kg, 17.0, 34.0)
        }
    }
}

/// Represents the weight in kg per area of the balance board.
#[derive(Debug, Clone)]
pub struct WeightValues {
    pub top_right: f32,
    pub bottom_right: f32,
    pub top_left: f32,
    pub bottom_left: f32,
    /// Current battery level
    /// - `0x00`: empty
    /// - `0x01` to `0x0E`: 1 bar
    /// - `0x0F` to `0x13`: 2 bars
    /// - `0x14` to `0x18`: 3 bars
    /// - `0x19` or greater: 4 bars
    pub battery: Option<u8>,
}

impl WeightValues {
    #[must_use]
    pub fn total(&self) -> f32 {
        self.top_right + self.bottom_right + self.top_left + self.bottom_left
    }
}

#[derive(Debug, Default, Clone)]
struct WeightData {
    top_right: u16,
    bottom_right: u16,
    top_left: u16,
    bottom_left: u16,
}

impl WeightData {
    fn read(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        Some(Self {
            top_right: u16::from_be_bytes([data[0], data[1]]),
            bottom_right: u16::from_be_bytes([data[2], data[3]]),
            top_left: u16::from_be_bytes([data[4], data[5]]),
            bottom_left: u16::from_be_bytes([data[6], data[7]]),
        })
    }
}

/// Represents the raw data received from the balance board.
///
/// Use `try_from` on the extension bytes to convert to this type.
/// Can be converted to `WeightValues` using the balance board calibration of the `WiimoteDevice`.
#[derive(Debug)]
pub struct BalanceBoardData {
    weights: WeightData,
    temperature: Option<u8>,
    battery: Option<u8>,
}

impl TryFrom<&[u8]> for BalanceBoardData {
    type Error = WiimoteDeviceError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        // https://www.wiibrew.org/wiki/Wii_Balance_Board#Data_Format
        WeightData::read(data).map_or(Err(WiimoteDeviceError::InvalidData), |weights| {
            Ok(Self {
                weights,
                temperature: if data.len() > 8 { Some(data[8]) } else { None },
                battery: if data.len() > 10 {
                    Some(data[10])
                } else {
                    None
                },
            })
        })
    }
}
