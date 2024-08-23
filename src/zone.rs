use core::mem;
use core::array;

#[derive(Clone, Copy)]
pub enum zone_type{
    ZONE_UNDEF,
    ZONE_NORMAL,
    ZONE_VIRTIO
}

const zone_cnt:usize = 10;

impl zone_type{
    pub fn as_str(&self) -> &str{
        match self{
            zone_type::ZONE_NORMAL => {
                "ZONE_NORMAL"
            },
            zone_type::ZONE_VIRTIO => {
                "ZONE_VIRTIO"
            },
            zone_type::ZONE_UNDEF => {
                "ZONE_UNDEF"
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct mem_zone{
    begin_addr: *mut u8,
    end_addr: *mut u8,
    zone_size: usize,
    types: zone_type,
}

impl mem_zone{
    pub fn new() -> Self{
        mem_zone{
            begin_addr: 0 as *mut u8,
            end_addr: 0 as *mut u8,
            zone_size: 0,
            types: zone_type::ZONE_UNDEF
        }
    }

    pub fn init(&mut self, _start: *mut u8, _end: *mut u8, _type: zone_type){
        self.begin_addr = _start;
        self.end_addr = _end;
        self.zone_size = (unsafe{_end.offset_from(_start)}) as usize;
        self.types = _type;
    }

    pub fn print_all(&self) {
        println!("[ZONE INFO] Begin: {} -> End: {}  Size: {}  Type: {:#?}",
            self.begin_addr as usize,
            self.end_addr as usize,
            self.zone_size,
            self.types.as_str());
    }
}

pub struct system_zones{
    next_zone: usize,
    zones: [mem_zone; zone_cnt]
}

impl system_zones{
    pub fn new() -> Self{
        system_zones{
            next_zone: 0,
            zones: [mem_zone::new(); zone_cnt]
        }
    }

    pub fn add_newzone(&mut self, zone_begin: *mut u8, zone_end:*mut u8, ztype:zone_type) {
        self.zones[self.next_zone].init(zone_begin, zone_end, ztype);
        self.next_zone += 1;
    }

    pub fn print_all(&self) {
        for i in 0..self.next_zone{
            self.zones[i].print_all();
        }
    }

    pub fn get_from_type(&mut self, target_type: zone_type) -> Option<&mut mem_zone>{
        for z in self.zones.iter_mut(){
            if let target_type = z.types{
                return Some(z);
            }
        }

        None
    }
}
