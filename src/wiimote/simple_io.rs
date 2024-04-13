use crate::prelude::*;

use super::input::{AcknowledgeData, MemoryData};

const RETRY_COUNT: usize = 5;
const READ_TIMEOUT: usize = 250;

/// Reads up to 16 bytes from the Wii remote.
/// Discards reports other than the expected data, only use during setup to prevent race-conditions.
pub fn read_16_bytes_sync(
    wiimote: &WiimoteDevice,
    addressing: Addressing,
) -> WiimoteResult<MemoryData> {
    let mut buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];

    let memory_read_request = OutputReport::ReadMemory(addressing);
    let size = memory_read_request.fill_buffer(false, &mut buffer);
    wiimote.write(&buffer[..size]).unwrap();

    for _i in 0..RETRY_COUNT {
        let size = wiimote.read_timeout(&mut buffer, READ_TIMEOUT)?;
        if size == 0 {
            return Err(WiimoteDeviceError::MissingData.into());
        }

        if let InputReport::ReadMemory(memory_data) = InputReport::try_from(buffer)? {
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
    let mut buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];

    let memory_write_request = OutputReport::WriteMemory(addressing, *data);
    let size = memory_write_request.fill_buffer(false, &mut buffer);
    wiimote.write(&buffer[..size]).unwrap();

    for _i in 0..RETRY_COUNT {
        let size = wiimote.read_timeout(&mut buffer, READ_TIMEOUT)?;
        if size == 0 {
            return Err(WiimoteDeviceError::MissingData.into());
        }

        let input_report = InputReport::try_from(buffer)?;
        if let InputReport::Acknowledge(acknowledge_data) = input_report {
            return Ok(acknowledge_data);
        }
    }
    Err(WiimoteDeviceError::InvalidData.into())
}
