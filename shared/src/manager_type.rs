/// Every data packet transmitted has data specific to either the Message,
/// Entity, Command managers. This value is written to differentiate those parts
/// of the payload.
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum ManagerType {
    /// A MessageManager
    Message = 1,
    /// A EntityManager
    Entity = 2,
    /// A CommandManager
    Command = 3,
    /// Unknown Manager
    Unknown = 255,
}

impl From<u8> for ManagerType {
    fn from(orig: u8) -> Self {
        match orig {
            1 => return ManagerType::Message,
            2 => return ManagerType::Entity,
            3 => return ManagerType::Command,
            _ => return ManagerType::Unknown,
        };
    }
}
