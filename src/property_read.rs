//! Inspect and update properties through the cEMI local device management interface.
//! The caller supplies the element width from the device profile and matches the peer/session.

/// Read request, positive confirmation or negative confirmation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropertyReadError {
    InvalidRequest,
    InvalidConfirmation,
    OtherRequest,
    UnsupportedProperty,
    AccessDenied,
    DeviceError(u8),
    ListTooLong,
    ListIncomplete,
    InstanceOutOfRange,
}

impl std::fmt::Display for PropertyReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidRequest => "Check the instance, element range and element width.",
            Self::InvalidConfirmation => {
                "Check the cEMI property confirmation and its payload width."
            }
            Self::OtherRequest => "Match the confirmation to the originating property request.",
            Self::UnsupportedProperty => "The device does not provide this property.",
            Self::AccessDenied => "The device denied access to this property.",
            Self::ListTooLong => "Interface object count exceeds the local cEMI read index range.",
            Self::ListIncomplete => {
                "Confirm every interface object list page before using an object index."
            }
            Self::InstanceOutOfRange => {
                "The object instance exceeds the local cEMI addressing range."
            }
            Self::DeviceError(_) => "The device returned a negative property confirmation.",
        })
    }
}
impl std::error::Error for PropertyReadError {}

/// A single read on a local management interface, with no device I/O.
///
/// ```
/// use knx::PropertyRead;
/// let serial_number = PropertyRead::new(0, 1, 11, 1, 1, 6).unwrap();
/// assert_eq!(serial_number.request(), [0xfc, 0, 0, 1, 11, 0x10, 1]);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PropertyRead {
    header: [u8; 7],
    expected_bytes: usize,
}

impl PropertyRead {
    /// Read 1–15 elements starting at a one-based 12-bit index. Index zero reads
    /// the property's element count as one unsigned two-byte value.
    pub fn new(
        object_type: u16,
        instance: u8,
        property: u8,
        start: u16,
        count: u8,
        element_width: u8,
    ) -> Result<Self, PropertyReadError> {
        if instance == 0
            || !(1..=15).contains(&count)
            || element_width == 0
            || u32::from(start) + u32::from(count) - 1 > 0xfff
            || (start == 0 && (count != 1 || element_width != 2))
        {
            return Err(PropertyReadError::InvalidRequest);
        }
        let [obj_hi, obj_lo] = object_type.to_be_bytes();
        let [index_hi, index_lo] = start.to_be_bytes();
        Ok(Self {
            header: [
                0xfc,
                obj_hi,
                obj_lo,
                instance,
                property,
                count << 4 | index_hi,
                index_lo,
            ],
            expected_bytes: usize::from(count) * usize::from(element_width),
        })
    }

    /// Seven-byte M_PropRead.req before transport encapsulation.
    pub const fn request(self) -> [u8; 7] {
        self.header
    }

    /// Return the entire requested element range, borrowed from M_PropRead.con.
    pub fn confirm(self, frame: &[u8]) -> Result<&[u8], PropertyReadError> {
        self.confirm_service(frame, 0xfb, self.expected_bytes)
    }

    fn confirm_service(
        self,
        frame: &[u8],
        service: u8,
        width: usize,
    ) -> Result<&[u8], PropertyReadError> {
        let Some((header, data)) = frame.split_at_checked(7) else {
            return Err(PropertyReadError::InvalidConfirmation);
        };
        if header[0] != service {
            return Err(PropertyReadError::InvalidConfirmation);
        }
        if header[1..5] != self.header[1..5]
            || header[5] & 0xf != self.header[5] & 0xf
            || header[6] != self.header[6]
        {
            return Err(PropertyReadError::OtherRequest);
        }
        if header[5] >> 4 == 0 {
            return Err(match data {
                [7] => PropertyReadError::UnsupportedProperty,
                [11] => PropertyReadError::AccessDenied,
                [code] => PropertyReadError::DeviceError(*code),
                _ => PropertyReadError::InvalidConfirmation,
            });
        }
        if header[5] != self.header[5] || data.len() != width {
            return Err(PropertyReadError::InvalidConfirmation);
        }
        Ok(data)
    }
}

/// Read the device object's PID_IO_LIST (71) on one local cEMI management connection.
///
/// Read index zero for the number of two-byte object types, then read one-based
/// array elements in pages. Keep this state with the same peer and session.
/// A rejected or partial confirmation leaves the current request pending.
#[derive(Debug)]
pub struct InterfaceObjectList {
    page_size: u8,
    total: Option<u16>,
    types: Vec<u16>,
}

impl Default for InterfaceObjectList {
    fn default() -> Self {
        Self {
            page_size: 15,
            total: None,
            types: Vec::new(),
        }
    }
}

impl InterfaceObjectList {
    /// Limit each data page to 1–15 elements, according to the local interface's payload limit.
    pub fn new(page_size: u8) -> Result<Self, PropertyReadError> {
        if !(1..=15).contains(&page_size) {
            return Err(PropertyReadError::InvalidRequest);
        }
        Ok(Self {
            page_size,
            ..Self::default()
        })
    }

    /// The next property read to send, or None when every advertised element was confirmed.
    pub fn next_request(&self) -> Option<PropertyRead> {
        let (start, count) = match self.total {
            None => (0, 1),
            Some(total) => {
                let received = self.types.len() as u16;
                let remaining = total - received;
                if remaining == 0 {
                    return None;
                }
                (received + 1, remaining.min(u16::from(self.page_size)) as u8)
            }
        };
        // Count and index are bounded by the constructor and the validated count confirmation.
        Some(PropertyRead::new(0, 1, 71, start, count, 2).expect("bounded PID_IO_LIST read"))
    }

    /// Accept exactly the pending count or data page. Device errors propagate without a fallback list.
    pub fn accept(&mut self, frame: &[u8]) -> Result<(), PropertyReadError> {
        let current = self.next_request().ok_or(PropertyReadError::OtherRequest)?;
        let data = current.confirm(frame)?;
        if self.total.is_none() {
            let count = u16::from_be_bytes(
                data.try_into()
                    .map_err(|_| PropertyReadError::InvalidConfirmation)?,
            );
            if count > 0xfff {
                return Err(PropertyReadError::ListTooLong);
            }
            self.total = Some(count);
        } else {
            self.types.extend(
                data.as_chunks::<2>()
                    .0
                    .iter()
                    .map(|word| u16::from_be_bytes(*word)),
            );
        }
        Ok(())
    }

    /// Ordered object types once enumeration completes. Slice positions are zero-based object indices.
    pub fn object_types(&self) -> Option<&[u16]> {
        self.total
            .filter(|&total| usize::from(total) == self.types.len())
            .map(|_| self.types.as_slice())
    }

    /// Address a property by the discovered object index. Each occurrence of a type has its
    /// own one-based instance number, including nonconsecutive occurrences.
    pub fn read_property(
        &self,
        object_index: usize,
        property: u8,
        start: u16,
        count: u8,
        element_width: u8,
    ) -> Result<PropertyRead, PropertyReadError> {
        let types = self
            .object_types()
            .ok_or(PropertyReadError::ListIncomplete)?;
        let object_type = *types
            .get(object_index)
            .ok_or(PropertyReadError::InvalidRequest)?;
        let occurrence = types[..=object_index]
            .iter()
            .filter(|&&kind| kind == object_type)
            .count();
        let instance =
            u8::try_from(occurrence).map_err(|_| PropertyReadError::InstanceOutOfRange)?;
        PropertyRead::new(object_type, instance, property, start, count, element_width)
    }
}

/// Progress of a read-modify-verify operation on a single local property range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropertyUpdateState {
    /// A fresh read is required before deciding whether to write.
    ReadRequired,
    /// A read was issued. Only its correlated confirmation may establish the value.
    AwaitingRead,
    /// The confirmed value differs. The host may authorize one write.
    Different,
    /// A write frame was issued. Delivery may be uncertain.
    WriteUnconfirmed,
    /// A read confirmed the desired bytes.
    Matches,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropertyUpdateError {
    InvalidSetting,
    UnexpectedState,
    AuthorizationDenied,
    Property(PropertyReadError),
}

impl std::fmt::Display for PropertyUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSetting => {
                f.write_str("Select readable property elements and supply their exact width.")
            }
            Self::UnexpectedState => {
                f.write_str("Check the pending operation and read back the setting before a write.")
            }
            Self::AuthorizationDenied => {
                f.write_str("The host policy denied this property update.")
            }
            Self::Property(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PropertyUpdateError {}

/// Set a readable, idempotent value on a commissioned local cEMI interface.
///
/// Bind this operation to the same device and serialize exchanges on its ordered
/// management connection. After losing a connection, discard its pending replies
/// and issue a fresh read to the same device before considering another write.
/// The caller owns transport authentication, access policy and device selection.
///
/// ```
/// use knx::{PropertyRead, PropertyUpdate, PropertyUpdateState};
/// let setting = PropertyRead::new(4, 1, 32, 1, 1, 1)?;
/// let mut update = PropertyUpdate::new(setting, &[3])?;
/// let _request = update.read_request();
/// let state = update.confirm_read(&[0xfb, 0, 4, 1, 32, 0x10, 1, 2])?;
/// if state == PropertyUpdateState::Different {
///     let write = update.write_request(|target, value| target == setting && value == [3])?;
///     assert_eq!(write, [0xf6, 0, 4, 1, 32, 0x10, 1, 3]);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct PropertyUpdate<'a> {
    target: PropertyRead,
    desired: &'a [u8],
    state: PropertyUpdateState,
}

impl<'a> PropertyUpdate<'a> {
    /// Select the elements and exact desired bytes. Array count writes are excluded.
    pub fn new(target: PropertyRead, desired: &'a [u8]) -> Result<Self, PropertyUpdateError> {
        if (target.header[5] & 0x0f == 0 && target.header[6] == 0)
            || desired.len() != target.expected_bytes
        {
            return Err(PropertyUpdateError::InvalidSetting);
        }
        Ok(Self {
            target,
            desired,
            state: PropertyUpdateState::ReadRequired,
        })
    }

    pub const fn state(&self) -> PropertyUpdateState {
        self.state
    }

    /// Start a read before the first write, after confirmation or after interruption.
    pub fn read_request(&mut self) -> [u8; 7] {
        self.state = PropertyUpdateState::AwaitingRead;
        self.target.request()
    }

    /// Return Matches or Different for a complete readback of the selected range.
    pub fn confirm_read(
        &mut self,
        frame: &[u8],
    ) -> Result<PropertyUpdateState, PropertyUpdateError> {
        if self.state != PropertyUpdateState::AwaitingRead {
            return Err(PropertyUpdateError::UnexpectedState);
        }
        let value = self
            .target
            .confirm(frame)
            .map_err(PropertyUpdateError::Property)?;
        self.state = if value == self.desired {
            PropertyUpdateState::Matches
        } else {
            PropertyUpdateState::Different
        };
        Ok(self.state)
    }

    /// Issue a single M_PropWrite.req after a different value was observed. Check the
    /// host's current permission for this exact device, property range and value in the callback.
    pub fn write_request(
        &mut self,
        authorize: impl FnOnce(PropertyRead, &[u8]) -> bool,
    ) -> Result<Vec<u8>, PropertyUpdateError> {
        if self.state != PropertyUpdateState::Different {
            return Err(PropertyUpdateError::UnexpectedState);
        }
        self.state = PropertyUpdateState::ReadRequired;
        if !authorize(self.target, self.desired) {
            return Err(PropertyUpdateError::AuthorizationDenied);
        }
        let mut frame = Vec::with_capacity(7 + self.desired.len());
        frame.extend_from_slice(&self.target.header);
        frame[0] = 0xf6;
        frame.extend_from_slice(self.desired);
        self.state = PropertyUpdateState::WriteUnconfirmed;
        Ok(frame)
    }

    /// Check the M_PropWrite.con header and status. Confirm the actual setting with a subsequent read.
    pub fn confirm_write(&mut self, frame: &[u8]) -> Result<(), PropertyUpdateError> {
        if self.state != PropertyUpdateState::WriteUnconfirmed {
            return Err(PropertyUpdateError::UnexpectedState);
        }
        match self.target.confirm_service(frame, 0xf5, 0) {
            Ok(_) => {
                self.state = PropertyUpdateState::ReadRequired;
                Ok(())
            }
            Err(e) => {
                if matches!(
                    e,
                    PropertyReadError::UnsupportedProperty
                        | PropertyReadError::AccessDenied
                        | PropertyReadError::DeviceError(_)
                ) {
                    self.state = PropertyUpdateState::ReadRequired;
                }
                Err(PropertyUpdateError::Property(e))
            }
        }
    }
}
