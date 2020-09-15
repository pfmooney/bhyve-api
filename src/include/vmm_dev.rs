//! Constants and structs for interfacing with the Bhyve ioctl interface.
//!
//! These are defined in Rust, but mimic the C constants and structs
//! defined in `machine/vmm_dev.h`, `sys/ioccom.h`, and `sys/time.h`.

use std::os::raw::{c_int, c_uint, c_long, c_longlong, c_ulonglong, c_char};
use std::mem::size_of;
use libc::{size_t, timeval};

use crate::include::vmm::*;

// Define const from sys/param.h

const SPECNAMELEN: usize = 255; // max length of devicename

// Define constants from machine/vmm_dev.h

const VMMCTL_IOC_BASE: i32 = ((b'V' as i32) << 16) | ((b'M' as i32)  << 8);
const VMM_IOC_BASE: i32 = ((b'v' as i32)  << 16) | ((b'm' as i32)  << 8);
const VMM_LOCK_IOC_BASE: i32 = ((b'v' as i32)  << 16) | ((b'l' as i32)  << 8);
const VMM_CPU_IOC_BASE: i32 = ((b'v' as i32)  << 16) | ((b'p' as i32)  << 8);

/* Operations performed on the vmmctl device */
pub const VMM_CREATE_VM: i32 = VMMCTL_IOC_BASE | 0x01;
pub const VMM_DESTROY_VM: i32 = VMMCTL_IOC_BASE | 0x02;
pub const VMM_VM_SUPPORTED: i32 = VMMCTL_IOC_BASE | 0x03;

/* Operations performed in the context of a given vCPU */
pub const VM_RUN: i32 = VMM_CPU_IOC_BASE | 0x01;
pub const VM_SET_REGISTER: i32 = VMM_CPU_IOC_BASE | 0x02;
pub const VM_GET_REGISTER: i32 = VMM_CPU_IOC_BASE | 0x03;
pub const VM_SET_SEGMENT_DESCRIPTOR: i32 = VMM_CPU_IOC_BASE | 0x04;
pub const VM_GET_SEGMENT_DESCRIPTOR: i32 = VMM_CPU_IOC_BASE | 0x05;
pub const VM_SET_REGISTER_SET: i32 = VMM_CPU_IOC_BASE | 0x06;
pub const VM_GET_REGISTER_SET: i32 = VMM_CPU_IOC_BASE | 0x07;
pub const VM_INJECT_EXCEPTION: i32 = VMM_CPU_IOC_BASE | 0x08;
pub const VM_SET_CAPABILITY: i32 = VMM_CPU_IOC_BASE | 0x09;
pub const VM_GET_CAPABILITY: i32 = VMM_CPU_IOC_BASE | 0x0a;
pub const VM_PPTDEV_MSI: i32 = VMM_CPU_IOC_BASE | 0x0b;
pub const VM_PPTDEV_MSIX: i32 = VMM_CPU_IOC_BASE | 0x0c;
pub const VM_SET_X2APIC_STATE: i32 = VMM_CPU_IOC_BASE | 0x0d;
pub const VM_GLA2GPA: i32 = VMM_CPU_IOC_BASE | 0x0e;
pub const VM_GLA2GPA_NOFAULT: i32 = VMM_CPU_IOC_BASE | 0x0f;
pub const VM_ACTIVATE_CPU: i32 = VMM_CPU_IOC_BASE | 0x10;
pub const VM_SET_INTINFO: i32 = VMM_CPU_IOC_BASE | 0x11;
pub const VM_GET_INTINFO: i32 = VMM_CPU_IOC_BASE | 0x12;
pub const VM_RESTART_INSTRUCTION: i32 = VMM_CPU_IOC_BASE | 0x13;
pub const VM_SET_KERNEMU_DEV: i32 = VMM_CPU_IOC_BASE | 0x14;
pub const VM_GET_KERNEMU_DEV: i32 = VMM_CPU_IOC_BASE | 0x15;

/* Operations requiring write-locking the VM */
pub const VM_REINIT: i32 = VMM_LOCK_IOC_BASE | 0x01;
pub const VM_BIND_PPTDEV: i32 = VMM_LOCK_IOC_BASE | 0x02;
pub const VM_UNBIND_PPTDEV: i32 = VMM_LOCK_IOC_BASE | 0x03;
pub const VM_MAP_PPTDEV_MMIO: i32 = VMM_LOCK_IOC_BASE | 0x04;
pub const VM_ALLOC_MEMSEG: i32 = VMM_LOCK_IOC_BASE | 0x05;
pub const VM_MMAP_MEMSEG: i32 = VMM_LOCK_IOC_BASE | 0x06;

pub const VM_WRLOCK_CYCLE: i32 = VMM_LOCK_IOC_BASE | 0xff;

/* All other ioctls */
pub const VM_GET_GPA_PMAP: i32 = VMM_IOC_BASE | 0x01;
pub const VM_GET_MEMSEG: i32 = VMM_IOC_BASE | 0x02;
pub const VM_MMAP_GETNEXT: i32 = VMM_IOC_BASE | 0x03;

pub const VM_LAPIC_IRQ: i32 = VMM_IOC_BASE | 0x04;
pub const VM_LAPIC_LOCAL_IRQ: i32 = VMM_IOC_BASE | 0x05;
pub const VM_LAPIC_MSI: i32 = VMM_IOC_BASE | 0x06;

pub const VM_IOAPIC_ASSERT_IRQ: i32 = VMM_IOC_BASE | 0x07;
pub const VM_IOAPIC_DEASSERT_IRQ: i32 = VMM_IOC_BASE | 0x08;
pub const VM_IOAPIC_PULSE_IRQ: i32 = VMM_IOC_BASE | 0x09;

pub const VM_ISA_ASSERT_IRQ: i32 = VMM_IOC_BASE | 0x0a;
pub const VM_ISA_DEASSERT_IRQ: i32 = VMM_IOC_BASE | 0x0b;
pub const VM_ISA_PULSE_IRQ: i32 = VMM_IOC_BASE | 0x0c;
pub const VM_ISA_SET_IRQ_TRIGGER: i32 = VMM_IOC_BASE | 0x0d;

pub const VM_RTC_WRITE: i32 = VMM_IOC_BASE | 0x0e;
pub const VM_RTC_READ: i32 = VMM_IOC_BASE | 0x0f;
pub const VM_RTC_SETTIME: i32 = VMM_IOC_BASE | 0x10;
pub const VM_RTC_GETTIME: i32 = VMM_IOC_BASE | 0x11;

pub const VM_SUSPEND: i32 = VMM_IOC_BASE | 0x12;

pub const VM_IOAPIC_PINCOUNT: i32 = VMM_IOC_BASE | 0x13;
pub const VM_GET_PPTDEV_LIMITS: i32 = VMM_IOC_BASE | 0x14;
pub const VM_GET_HPET_CAPABILITIES: i32 = VMM_IOC_BASE | 0x15;

pub const VM_STATS_IOC: i32 = VMM_IOC_BASE | 0x16;
pub const VM_STAT_DESC: i32 = VMM_IOC_BASE | 0x17;

pub const VM_INJECT_NMI: i32 = VMM_IOC_BASE | 0x18;
pub const VM_GET_X2APIC_STATE: i32 = VMM_IOC_BASE | 0x19;
pub const VM_SET_TOPOLOGY: i32 = VMM_IOC_BASE | 0x1a;
pub const VM_GET_TOPOLOGY: i32 = VMM_IOC_BASE | 0x1b;
pub const VM_GET_CPUS: i32 = VMM_IOC_BASE | 0x1c;
pub const VM_SUSPEND_CPU: i32 = VMM_IOC_BASE | 0x1d;
pub const VM_RESUME_CPU: i32 = VMM_IOC_BASE | 0x1e;


pub const VM_DEVMEM_GETOFFSET: i32 = VMM_IOC_BASE | 0xff;


// Define structs from machine/vmm_dev.h

// For VM_MMAP_MEMSEG
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_memmap {
    pub gpa: c_ulonglong,
    pub segid: c_int,            // memory segment
    pub segoff: c_longlong,      // offset into memory segment
    pub len: size_t,             // mmap length
    pub prot: c_int,             // RWX
    pub flags: c_int,
}

pub const VM_MEMMAP_F_WIRED: c_int = 0x01;
#[allow(unused)]
pub const VM_MEMMAP_F_IOMMU: c_int = 0x02;

// For VM_MUNMAP_MEMSEG
#[repr(C)]
pub struct vm_munmap {
    pub gpa: c_ulonglong,
    pub len: size_t,
}


// For VM_ALLOC_MEMSEG and VM_GET_MEMSEG
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_memseg {
    pub segid: c_int,
    pub len: size_t,
    pub name: [c_char; SPECNAMELEN + 1],
}

impl Default for vm_memseg {
    fn default() -> vm_memseg {
        vm_memseg {
            segid: 0,
            len: 0,
            name: [0 as c_char; SPECNAMELEN + 1],
        }
    }
}

// For VM_RTC_SETTIME and VM_RTC_GETTIME
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_rtc_time {
    pub secs: c_longlong,
}

// For VM_RTC_WRITE and VM_RTC_READ
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_rtc_data {
    pub offset: c_int,
    pub value: u8,
}

// For VM_DEVMEM_GETOFFSET
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_devmem_offset {
    pub segid: c_int,
    pub offset: c_longlong,
}

// For VM_SET_REGISTER and VM_GET_REGISTER
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_register {
    pub cpuid: c_int,
    pub regnum: c_int,      // enum vm_reg_name
    pub regval: c_ulonglong,
}

// For VM_SET_SEGMENT_DESCRIPTOR and VM_GET_SEGMENT_DESCRIPTOR
// data or code segment
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_seg_desc {
    pub cpuid: c_int,
    pub regnum: c_int,      // enum vm_reg_name
    pub desc: seg_desc,     // struct seg_desc
}

// For VM_RUN
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_run {
    pub cpuid: c_int,
    pub vm_exit: vm_exit,
}

// For VM_SET_CAPABILITY and VM_GET_CAPABILITY
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_capability {
    pub cpuid: c_int,
    pub captype: vm_cap_type, // enum vm_cap_type
    pub capval: c_int,
    pub allcpus: c_int,
}

impl Default for vm_capability {
    fn default() -> vm_capability {
        vm_capability {
            cpuid: 0,
            captype: vm_cap_type::VM_CAP_MAX,
            capval: 0,
            allcpus: 0,
        }
    }
}

// For VM_GET_X2APIC_STATE and VM_SET_X2APIC_STATE
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_x2apic {
    pub cpuid: c_int,
    pub state: x2apic_state,
}

impl Default for vm_x2apic {
    fn default() -> vm_x2apic {
        vm_x2apic {
            cpuid: 0,
            state: x2apic_state::X2APIC_DISABLED,
        }
    }
}

// For VM_SUSPEND
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_suspend {
    pub how: vm_suspend_how,
}

// For VM_ACTIVATE_CPU, VM_SUSPEND_CPU, and VM_RESUME_CPU
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_activate_cpu {
    pub vcpuid: c_int,
}

// For VM_SET_TOPOLOGY and VM_GET_TOPOLOGY
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_cpu_topology {
    pub sockets: u16,
    pub cores: u16,
    pub threads: u16,
    pub maxcpus: u16,
}

const MAX_VM_STATS: usize = 64 + VM_MAXCPU;

// For VM_STATS_IOC
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_stats {
    pub cpuid: c_int,       // in
    pub num_entries: c_int, // out
    pub tv: timeval,
    pub statbuf: [c_ulonglong; MAX_VM_STATS],
}

impl Default for vm_stats {
    fn default() -> vm_stats {
        vm_stats {
            cpuid: 0,
            num_entries: 0,
            tv: timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            statbuf: [0; MAX_VM_STATS],
        }
    }
}

// For VM_SET_INTINFO and VM_GET_INTINFO
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_intinfo {
    pub vcpuid: c_int,
    pub info1: c_ulonglong,
    pub info2: c_ulonglong,
}

// For VM_INJECT_EXCEPTION
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_exception {
    pub cpuid: c_int,
    pub vector: c_int,
    pub error_code: c_uint,
    pub error_code_valid: c_int,
    pub restart_instruction: c_int,
}

// For VM_INJECT_NMI
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_nmi {
    pub cpuid: c_int,
}

// For VM_LAPIC_MSI
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_lapic_msi {
    pub msg: c_ulonglong,
    pub addr: c_ulonglong,
}

// For VM_LAPIC_IRQ and VM_LAPIC_LOCAL_IRQ
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_lapic_irq {
    pub cpuid: c_int,
    pub vector: c_int,
}

// For VM_IOAPIC_ASSERT_IRQ, VM_IOAPIC_DEASSERT_IRQ, and VM_IOAPIC_PULSE_IRQ
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_ioapic_irq {
    pub irq: c_int,
}


#[cfg(test)]
mod tests {
    use crate::include::vmm_dev::*;

    #[test]
    fn test_ioctl_stats() {
        assert_eq!(size_of::<vm_stats>(), 0x318);
        assert_eq!(VM_STATS_IOC as u32, 0xc0187632);
    }

    #[test]
    fn test_ioctl_general() {
        assert_eq!(size_of::<vm_run>(), 0x90);
        assert_eq!(size_of::<vm_suspend>(), 4);

        //assert_eq!(VM_RUN as u32, 0xc0847601);
        assert_eq!(VM_RUN as u32, 0xc0907601);
        assert_eq!(VM_SUSPEND as u32, 0x80047604);
        assert_eq!(VM_REINIT as u32, 0x20007605);
    }

    #[test]
    fn test_ioctl_topology() {
        assert_eq!(size_of::<vm_activate_cpu>(), 4);
        assert_eq!(size_of::<vm_cpu_topology>(), 8);
        assert_eq!(VM_SET_TOPOLOGY as u32, 0x8008763f);
        assert_eq!(VM_GET_TOPOLOGY as u32, 0x40087640);
    }

    #[test]
    fn test_ioctl_memory() {
        assert_eq!(size_of::<vm_memseg>(), 0x110);
        assert_eq!(size_of::<vm_memmap>(), 0x28);
        assert_eq!(VM_ALLOC_MEMSEG as u32, 0x8010760E);
        assert_eq!(VM_MMAP_MEMSEG as u32, 0x80287610);
        assert_eq!(VM_MMAP_GETNEXT as u32, 0xc0287611);
    }
}
