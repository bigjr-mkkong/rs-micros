use core::mem;
use core::array;

#[derive(Clone, Copy)]
pub enum zone_type{
    ZONE_UNDEF,
    ZONE_NORMAL,
    ZONE_MMIO
}

const zone_cnt:usize = 10;

impl zone_type{
    pub fn as_str(&self) -> &str{
        match self{
            zone_type::ZONE_NORMAL => {
                "ZONE_NORMAL"
            },
            zone_type::ZONE_MMIO => {
                "ZONE_MMIO"
            },
            zone_type::ZONE_UNDEF => {
                "ZONE_UNDEF"
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct mem_zone{
    begin_addr: usize,
    end_addr: usize,
    zone_size: usize,
    types: zone_type,
}

impl mem_zone{
    pub fn new() -> Self{
        mem_zone{
            begin_addr: 0,
            end_addr: 0,
            zone_size: 0,
            types: zone_type::ZONE_UNDEF
        }
    }

    pub fn init(&mut self, _start: usize, _size: usize, _type: zone_type){
        self.begin_addr = _start;
        self.zone_size = _size;
        self.end_addr = _start + _size;
        self.types = _type;
    }

    pub fn print_all(&self) {
        println!("Begin: {} -- End: {}  Size: {}  Type: {:#?}",
            self.begin_addr,
            self.end_addr,
            self.zone_size,
            self.types.as_str())
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

    pub fn add_newzone(&mut self, zone_begin: usize, zone_size:usize, ztype:zone_type) {
        self.zones[self.next_zone].init(zone_begin, zone_size, ztype);
        self.next_zone += 1;
    }

    pub fn print_all(&self) {
        for i in 0..self.next_zone{
            self.zones[i].print_all();
        }
    }
}
