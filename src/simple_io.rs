use crate::prelude::*;

use crate::input::{AcknowledgeData, InputReport, MemoryData};
use crate::output::{Addressing, OutputReport};

const RETRY_COUNT: usize = 5;
const READ_TIMEOUT: usize = 250;

/// Reads up to 16 bytes from the Wii remote.
/// Discards reports other than the expected data, only use during setup to prevent race-conditions.
pub fn read_16_bytes_sync(
    wiimote: &WiimoteDevice,
    addressing: Addressing,
) -> WiimoteResult<MemoryData> {
    let memory_read_request = OutputReport::ReadMemory(addressing);
    wiimote.write(&memory_read_request).unwrap();

    for _i in 0..RETRY_COUNT {
        let input_report = wiimote.read_timeout(READ_TIMEOUT)?;
        if let InputReport::ReadMemory(memory_data) = input_report {
            return Ok(memory_data);
        }
    }
    Err(WiimoteDeviceError::InvalidData.into())
}

/// Reads up to 16 bytes from the Wii remote and checks the resulting report data.
/// Discards reports other than the expected data, only use during setup to prevent race-conditions.
pub fn read_16_bytes_sync_checked(
    wiimote: &WiimoteDevice,
    addressing: Addressing,
) -> WiimoteResult<[u8; 16]> {
    let address = addressing.address;
    let size = addressing.size;

    let memory_data = read_16_bytes_sync(wiimote, addressing)?;
    if memory_data.address_offset() != address as u16 || (memory_data.size() as u16) < size {
        Err(WiimoteDeviceError::InvalidData.into())
    } else {
        Ok(memory_data.data)
    }
}

/// Writes up to 16 bytes to the Wii remote.
/// Discards reports other than the acknowledge result, only use during setup to prevent race-conditions.
pub fn write_16_bytes_sync(
    wiimote: &WiimoteDevice,
    addressing: Addressing,
    data: &[u8; 16],
) -> WiimoteResult<AcknowledgeData> {
    let memory_write_request = OutputReport::WriteMemory(addressing, *data);
    wiimote.write(&memory_write_request).unwrap();

    for _i in 0..RETRY_COUNT {
        let input_report = wiimote.read_timeout(READ_TIMEOUT)?;
        if let InputReport::Acknowledge(acknowledge_data) = input_report {
            return Ok(acknowledge_data);
        }
    }
    Err(WiimoteDeviceError::InvalidData.into())
}
