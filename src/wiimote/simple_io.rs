use crate::prelude::*;

use super::input::{AcknowledgeData, MemoryData};

/// Reads up to 16 bytes from the Wii remote.
/// Expects the next report to be the data, only use during setup to prevent race-conditions.
pub fn read_16_bytes_sync(
    wiimote: &WiimoteDevice,
    addressing: Addressing,
) -> WiimoteResult<MemoryData> {
    let mut buffer = [0u8; WIIMOTE_REPORT_BUFFER_SIZE];

    let memory_read_request = OutputReport::ReadMemory(addressing);
    let size = memory_read_request.fill_buffer(false, &mut buffer);
    wiimote.write(&buffer[..size]).unwrap();

    let size = wiimote.read_timeout(&mut buffer, 100)?;
    if size == 0 {
        return Err(WiimoteDeviceError::MissingData.into());
    }

    if let InputReport::ReadMemory(memory_data) = InputReport::try_from(buffer)? {
        Ok(memory_data)
    } else {
        Err(WiimoteDeviceError::InvalidData.into())
    }
}

/// Reads up to 16 bytes from the Wii remote and checks the resulting report data.
/// Expects the next report to be the data, only use during setup to prevent race-conditions.
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
/// Expects the next report to be the acknowledge result, only use during setup to prevent race-conditions.
pub fn write_16_bytes_sync(
    wiimote: &WiimoteDevice,
    addressing: Addressing,
    data: &[u8; 16],
) -> WiimoteResult<AcknowledgeData> {
    let mut buffer = [0u8; WIIMOTE_REPORT_BUFFER_SIZE];

    let memory_write_request = OutputReport::WriteMemory(addressing, *data);
    let size = memory_write_request.fill_buffer(false, &mut buffer);
    wiimote.write(&buffer[..size]).unwrap();

    let size = wiimote.read_timeout(&mut buffer, 100)?;
    if size == 0 {
        return Err(WiimoteDeviceError::MissingData.into());
    }

    let input_report = InputReport::try_from(buffer)?;
    if let InputReport::Acknowledge(acknowledge_data) = input_report {
        Ok(acknowledge_data)
    } else {
        Err(WiimoteDeviceError::InvalidData.into())
    }
}
