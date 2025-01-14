use naia_derive::Replicate;
use naia_shared::Property;

#[derive(Replicate)]
#[protocol_path = "crate::protocol::Protocol"]
pub struct StringMessage {
    pub contents: Property<String>,
}

impl StringMessage {
    pub fn new(contents: String) -> Self {
        return StringMessage::new_complete(contents);
    }
}
