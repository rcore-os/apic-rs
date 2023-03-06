use super::LocalApic;
use x86::cpuid::CpuId;
use x86::msr::*;

#[derive(Default)]
pub struct X2Apic;

impl LocalApic for X2Apic {
    fn support() -> bool {
        CpuId::new().get_feature_info().unwrap().has_x2apic()
    }

    fn cpu_init(&mut self) {
        unsafe {
            wrmsr(IA32_APIC_BASE, rdmsr(IA32_APIC_BASE) | 1 << 10);
            wrmsr(IA32_X2APIC_SIVR, 0x100);
        }
    }

    fn id(&self) -> u32 {
        unsafe { rdmsr(IA32_X2APIC_APICID) as u32 }
    }

    fn version(&self) -> u32 {
        unsafe { rdmsr(IA32_X2APIC_VERSION) as u32 }
    }

    fn icr(&self) -> u64 {
        unsafe { rdmsr(IA32_X2APIC_ICR) }
    }

    fn set_icr(&mut self, value: u64) {
        unsafe {
            wrmsr(IA32_X2APIC_ICR, value);
        }
    }

    fn eoi(&mut self) {
        unsafe {
            wrmsr(IA32_X2APIC_EOI, 0);
        }
    }

    fn send_ipi(&mut self, apic_id: u8, int_id: u8) {
        self.set_icr((apic_id as u64) << 32 | int_id as u64 | 1 << 14);
    }

    unsafe fn start_ap(&mut self, _apic_id: u8, _addr: u32) {
        unimplemented!()
    }
}
