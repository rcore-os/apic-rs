use super::LocalApic;
use x86::io::outb;
use x86::cpuid::CpuId;
use core::ptr::{read_volatile, write_volatile};
use core::fmt::{Debug, Formatter, Error};

pub struct XApic {
    addr: usize
}

impl XApic {
    unsafe fn read(&self, reg: u32) -> u32 {
        read_volatile((self.addr + reg as usize) as *const u32)
    }

    unsafe fn write(&mut self, reg: u32, value: u32) {
        write_volatile((self.addr + reg as usize) as *mut u32, value);
        self.read(0x20); // wait for write to finish, by reading
    }
}

impl XApic {
    pub fn new(addr: usize) -> Self {
        XApic { addr }
    }
}

impl Default for XApic {
    fn default() -> Self {
        XApic { addr: 0xfee00000 }
    }
}

impl LocalApic for XApic {
    fn support() -> bool {
        CpuId::new().get_feature_info().unwrap().has_apic()
    }

    fn cpu_init(&mut self) {
        unsafe { self.write(0xF0, 0x100); }
    }

    fn id(&self) -> u32 {
        unsafe { self.read(0x20) >> 24 }
    }

    fn version(&self) -> u32 {
        unsafe { self.read(0x30) }
    }

    fn icr(&self) -> u64 {
        unsafe { (self.read(0x310) as u64) << 32 | self.read(0x300) as u64 }
    }

    fn set_icr(&mut self, value: u64) {
        unsafe {
            while self.read(0x300) & 1 << 12 == 1 << 12 {}
            self.write(0x310, (value >> 32) as u32);
            self.write(0x300, value as u32);
            while self.read(0x300) & 1 << 12 == 1 << 12 {}
        }
    }

    unsafe fn eoi(&mut self) {
        self.write(0xB0, 0);
    }

    fn send_ipi(&mut self, apic_id: u8) {
        self.set_icr(0x4040 | (apic_id as u64) << 56);
    }

    /// The entry point `addr` must be 4K aligned.
    /// This function will access memory: 0x467
    unsafe fn start_ap(&mut self, apic_id: u8, addr: u32) {
        const CMOS_PORT: u16 = 0x70;
        const CMOS_RETURN: u16 = 0x71;
        const ICRLO: u32 = 0x0300;          // Interrupt Command
        const ICRHI: u32 = 0x0310;          // Interrupt Command [63:32]
        const INIT: u32 = 0x00000500;       // INIT/RESET
        const STARTUP: u32 = 0x00000600;    // Startup IPI
        const DELIVS: u32 = 0x00001000;     // Delivery status
        const ASSERT: u32 = 0x00004000;     // Assert interrupt (vs deassert)
        const DEASSERT: u32 = 0x00000000;
        const LEVEL: u32 = 0x00008000;      // Level triggered
        const BCAST: u32 = 0x00080000;      // Send to all APICs, including self.
        const BUSY: u32 = 0x00001000;
        const FIXED: u32 = 0x00000000;

        // "The BSP must initialize CMOS shutdown code to 0AH
        // and the warm reset vector (DWORD based at 40:67) to point at
        // the AP startup code prior to the [universal startup algorithm]."
        outb(CMOS_PORT, 0xf);   // offset 0xF is shutdown code
        outb(CMOS_RETURN, 0xa);

        let wrv = (0x40 << 4 | 0x67) as *mut u16;  // Warm reset vector
        *wrv = 0;
        *wrv.offset(1) = addr as u16 >> 4;

        // "Universal startup algorithm."
        // Send INIT (level-triggered) interrupt to reset other CPU.
        self.write(ICRHI, (apic_id as u32) << 24);
        self.write(ICRLO, INIT | LEVEL | ASSERT);
        microdelay(200);
        self.write(ICRLO, INIT | LEVEL);
        microdelay(10000);

        // Send startup IPI (twice!) to enter code.
        // Regular hardware is supposed to only accept a STARTUP
        // when it is in the halted state due to an INIT.  So the second
        // should be ignored, but it is part of the official Intel algorithm.
        for _ in 0..2 {
            self.write(ICRHI, (apic_id as u32) << 24);
            self.write(ICRLO, STARTUP | (addr >> 12) as u32);
            microdelay(200);
        }
    }
}

impl Debug for XApic {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Xapic")
            .field("id", &self.id())
            .field("version", &self.version())
            .field("icr", &self.icr())
            .finish()
    }
}

fn microdelay(_ms: usize) {
    // TODO: micro delay
}