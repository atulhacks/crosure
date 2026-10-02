use std::collections::BTreeMap;

use object::pe::{ImageNtHeaders32, ImageNtHeaders64};
use object::read::pe::{ImageNtHeaders, ImageOptionalHeader, Import, PeFile};
use object::LittleEndian as LE;

/// PE: records the virtual address of every IAT slot with its import name.
pub(crate) fn iat_slots(data: &[u8], is_64: bool, out: &mut BTreeMap<u64, String>) {
    if is_64 {
        collect::<ImageNtHeaders64>(data, 8, out);
    } else {
        collect::<ImageNtHeaders32>(data, 4, out);
    }
}

fn collect<Pe: ImageNtHeaders>(data: &[u8], width: u64, out: &mut BTreeMap<u64, String>) {
    let Ok(pe) = PeFile::<Pe>::parse(data) else {
        return;
    };
    let base = pe.nt_headers().optional_header().image_base();
    let Ok(Some(table)) = pe.import_table() else {
        return;
    };
    let Ok(mut descs) = table.descriptors() else {
        return;
    };
    while let Ok(Some(desc)) = descs.next() {
        let iat = desc.first_thunk.get(LE);
        let lookup = match desc.original_first_thunk.get(LE) {
            0 => iat,
            rva => rva,
        };
        let Ok(mut thunks) = table.thunks(lookup) else {
            continue;
        };
        let mut index = 0u64;
        while let Ok(Some(thunk)) = thunks.next::<Pe>() {
            let name = match table.import::<Pe>(thunk) {
                Ok(Import::Name(_, n)) => String::from_utf8_lossy(n).into_owned(),
                Ok(Import::Ordinal(o)) => format!("ordinal_{o}"),
                Err(_) => break,
            };
            out.insert(base + u64::from(iat) + index * width, name);
            index += 1;
        }
    }
}
