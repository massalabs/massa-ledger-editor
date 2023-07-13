#[allow(unused_imports)]
use std::collections::BTreeMap;

#[allow(unused_imports)]
use massa_time::MassaTime;

#[allow(unused_imports)]
use massa_versioning::versioning::{MipComponent, MipInfo, MipState};

use massa_versioning::mips::get_mip_list as versioning_get_mip_list;

pub fn get_mip_list() -> [(MipInfo, MipState); 0] {
    versioning_get_mip_list()
}
