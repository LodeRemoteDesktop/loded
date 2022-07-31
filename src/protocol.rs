use std::{mem::MaybeUninit, sync::Arc};

static PACKET_LENGTHS: [u64; 4] = [
    std::mem::size_of::<LodestarHandshakePacket>() as u64,
    0,
    std::mem::size_of::<LodestarSwitchSourcePacket>() as u64,
    std::mem::size_of::<LodestarEndPacket>() as u64,
];

#[repr(u64)]
#[derive(Clone, Copy)]
pub enum LodestarPacketType {
    Handshake,
    DesktopList,
    SwitchSource,
    End,
}

#[derive(Debug, thiserror::Error)]
pub enum LodestarPacketParsingError {
    #[error("The packet length was too short or long for the desired type")]
    InvalidPacketLength,
    #[error("An invalid field value was parsed")]
    InvalidField,
}

#[repr(C)]
pub struct LodestarPacket {
    packet_type: LodestarPacketType,
    packet_length: u64,
    packet_data: [u8],
}

impl LodestarPacket {
    pub fn parse_packet<T>(&self) -> std::result::Result<&T, LodestarPacketParsingError> {
        let ex_len = PACKET_LENGTHS[self.packet_type as u64 as usize];
        if self.packet_length == ex_len || ex_len == 0 {
            unsafe {
                Ok(&*(core::ptr::slice_from_raw_parts(
                    self.packet_data.as_ptr().cast::<()>(),
                    self.packet_length as usize,
                ) as *const T))
            }
        } else {
            Err(LodestarPacketParsingError::InvalidPacketLength)
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LodestarHandshakePacket {
    api_revision: u64,
    accepted: bool,
}

impl Into<Arc<[u8]>> for LodestarHandshakePacket {
    fn into(self) -> Arc<[u8]> {
        let mut data: Arc<[MaybeUninit<u8>]> = Arc::new_uninit_slice(2 * 8);
        let dataw = Arc::get_mut(&mut data).unwrap();
        for (idx, item) in self
            .api_revision
            .to_le_bytes()
            .iter()
            .chain((self.accepted as u64).to_le_bytes().iter())
            .enumerate()
        {
            dataw[idx].write(*item);
        }

        unsafe { data.assume_init() }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LodestarDesktop {
    loded_id: u64,
    width: i32,
    height: i32,
}

#[repr(C)]
pub struct LodestarDesktopPacket {
    desktop_count: u64,
    data: [LodestarDesktop],
}

impl LodestarDesktopPacket {
    pub fn into(&self) -> Arc<[u8]> {
        let mut data: Arc<[MaybeUninit<u8>]> = Arc::new_uninit_slice(
            8 + (std::mem::size_of::<LodestarDesktop>() * self.desktop_count as usize),
        );
        let dataw = Arc::get_mut(&mut data).unwrap();

        for (idx, item) in self.desktop_count.to_le_bytes().iter().enumerate() {
            dataw[idx].write(*item);
        }

        let slice =
            unsafe { std::slice::from_raw_parts(self.data.as_ptr(), self.desktop_count as usize) };
        for (idx, item) in slice.iter().enumerate() {
            for item in item
                .loded_id
                .to_le_bytes()
                .iter()
                .chain(item.width.to_le_bytes().iter())
                .chain(item.height.to_le_bytes().iter())
            {
                dataw[idx + 1].write(*item);
            }
        }

        unsafe { data.assume_init() }
    }
}

impl LodestarDesktopPacket {
    pub fn get_desktops(&self) -> &[LodestarDesktop] {
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr() as *const LodestarDesktop,
                self.desktop_count as usize,
            )
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LodestarSwitchSourcePacket {
    new_source: u64,
}

impl Into<Arc<[u8]>> for LodestarSwitchSourcePacket {
    fn into(self) -> Arc<[u8]> {
        let mut data: Arc<[MaybeUninit<u8>]> = Arc::new_uninit_slice(8);
        let dataw = Arc::get_mut(&mut data).unwrap();

        for (idx, item) in self.new_source.to_le_bytes().iter().enumerate() {
            dataw[idx].write(*item);
        }

        unsafe { data.assume_init() }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LodestarEndPacket {}

impl Into<Arc<[u8]>> for LodestarEndPacket {
    fn into(self) -> Arc<[u8]> {
        unsafe { Arc::new_uninit_slice(0).assume_init() }
    }
}
