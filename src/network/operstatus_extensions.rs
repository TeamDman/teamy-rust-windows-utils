use std::borrow::Cow;
use windows::Win32::NetworkManagement::Ndis::IF_OPER_STATUS;

pub trait OperStatusExt {
    fn display(&self) -> Cow<'_, str>;
}
impl OperStatusExt for IF_OPER_STATUS {
    fn display(&self) -> Cow<'_, str> {
        match self.0 {
            1 => Cow::Borrowed("Up"),
            2 => Cow::Borrowed("Down"),
            3 => Cow::Borrowed("Testing"),
            4 => Cow::Borrowed("Unknown"),
            5 => Cow::Borrowed("Dormant"),
            6 => Cow::Borrowed("NotPresent"),
            7 => Cow::Borrowed("LowerLayerDown"),
            x => Cow::Owned(format!("InvalidStatus({x})")),
        }
    }
}
