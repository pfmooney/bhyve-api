//! Constants and structs for interfacing with the Bhyve ioctl interface.
//!
//! These are defined in Rust, but mimic the C constants and structs
//! defined in `machine/vmm.h`.

use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

use num_enum::TryFromPrimitive;

pub const VM_MAXCPU: usize = 32;    // maximum virtual cpus

#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone)]
pub enum vm_suspend_how {
        VM_SUSPEND_NONE,
        VM_SUSPEND_RESET,
        VM_SUSPEND_POWEROFF,
        VM_SUSPEND_HALT,
        VM_SUSPEND_TRIPLEFAULT,
        VM_SUSPEND_LAST
}

// Identifiers for architecturally defined registers.
#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone)]
pub enum vm_reg_name {
        VM_REG_GUEST_RAX,
        VM_REG_GUEST_RBX,
        VM_REG_GUEST_RCX,
        VM_REG_GUEST_RDX,
        VM_REG_GUEST_RSI,
        VM_REG_GUEST_RDI,
        VM_REG_GUEST_RBP,
        VM_REG_GUEST_R8,
        VM_REG_GUEST_R9,
        VM_REG_GUEST_R10,
        VM_REG_GUEST_R11,
        VM_REG_GUEST_R12,
        VM_REG_GUEST_R13,
        VM_REG_GUEST_R14,
        VM_REG_GUEST_R15,
        VM_REG_GUEST_CR0,
        VM_REG_GUEST_CR3,
        VM_REG_GUEST_CR4,
        VM_REG_GUEST_DR7,
        VM_REG_GUEST_RSP,
        VM_REG_GUEST_RIP,
        VM_REG_GUEST_RFLAGS,
        VM_REG_GUEST_ES,
        VM_REG_GUEST_CS,
        VM_REG_GUEST_SS,
        VM_REG_GUEST_DS,
        VM_REG_GUEST_FS,
        VM_REG_GUEST_GS,
        VM_REG_GUEST_LDTR,
        VM_REG_GUEST_TR,
        VM_REG_GUEST_IDTR,
        VM_REG_GUEST_GDTR,
        VM_REG_GUEST_EFER,
        VM_REG_GUEST_CR2,
        VM_REG_GUEST_PDPTE0,
        VM_REG_GUEST_PDPTE1,
        VM_REG_GUEST_PDPTE2,
        VM_REG_GUEST_PDPTE3,
        VM_REG_GUEST_INTR_SHADOW,
        VM_REG_GUEST_DR0,
        VM_REG_GUEST_DR1,
        VM_REG_GUEST_DR2,
        VM_REG_GUEST_DR3,
        VM_REG_GUEST_DR6,
        VM_REG_GUEST_ENTRY_INST_LENGTH,
        VM_REG_LAST
}

#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone)]
pub enum x2apic_state {
	X2APIC_DISABLED,
	X2APIC_ENABLED,
	X2APIC_STATE_LAST
}

// Identifiers for optional vmm capabilities
#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone)]
pub enum vm_cap_type {
	VM_CAP_HALT_EXIT,
	VM_CAP_MTRAP_EXIT,
	VM_CAP_PAUSE_EXIT,
	VM_CAP_UNRESTRICTED_GUEST,
	VM_CAP_ENABLE_INVPCID,
	VM_CAP_MAX
}


// The 'access' field has the format specified in Table 21-2 of the Intel
// Architecture Manual vol 3b.
//
// XXX The contents of the 'access' field are architecturally defined except
// bit 16 - Segment Unusable.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct seg_desc {
    pub base: c_ulonglong,
    pub limit: c_uint,
    pub access: c_uint,
}

#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone)]
pub enum vm_cpu_mode {
        CPU_MODE_REAL,
        CPU_MODE_PROTECTED,
        CPU_MODE_COMPATIBILITY,         /* IA-32E mode (CS.L = 0) */
        CPU_MODE_64BIT,                 /* IA-32E mode (CS.L = 1) */
}

#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone)]
pub enum vm_paging_mode {
        PAGING_MODE_FLAT,
        PAGING_MODE_32,
        PAGING_MODE_PAE,
        PAGING_MODE_64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_guest_paging {
    pub gpa: u64,
    pub fault_type: c_int,
}


// Kernel-internal MMIO decoding and emulation.
// Userspace should not expect to see this, but rather a
// VM_EXITCODE_MMIO with the above 'mmio' context.
#[repr(C)]
#[allow(unused)]
struct mmio_emul{
    gpa: u64,
    gla: u64,
    cs_base: u64,
    cs_d: c_int,
}

#[repr(i32)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone, Debug, TryFromPrimitive)]
pub enum vm_exitcode {
        VM_EXITCODE_INOUT,
        VM_EXITCODE_VMX,
        VM_EXITCODE_BOGUS,
        VM_EXITCODE_RDMSR,
        VM_EXITCODE_WRMSR,
        VM_EXITCODE_HLT,
        VM_EXITCODE_MTRAP,
        VM_EXITCODE_PAUSE,
        VM_EXITCODE_PAGING,
        VM_EXITCODE_INST_EMUL,
        VM_EXITCODE_SPINUP_AP,
        VM_EXITCODE_MMIO_EMUL,
        VM_EXITCODE_RUNBLOCK,
        VM_EXITCODE_IOAPIC_EOI,
        VM_EXITCODE_SUSPENDED,
        VM_EXITCODE_MMIO,
        VM_EXITCODE_TASK_SWITCH,
        VM_EXITCODE_MONITOR,
        VM_EXITCODE_MWAIT,
        VM_EXITCODE_SVM,
        VM_EXITCODE_REQIDLE,
        VM_EXITCODE_DEBUG,
        VM_EXITCODE_VMINSN,
        VM_EXITCODE_BPT,
        VM_EXITCODE_HT,
}

#[repr(u32)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone)]
pub enum vm_entry_cmds {
    VEC_DEFAULT = 0,
    VEC_DISCARD_INSTR,
    VEC_COMPLETE_MMIO,
    VEC_COMPLETE_INOUT,
}

const INOUT_IN: u8 = 1 << 0;
#[allow(unused)]
const INOUT_STR: u8 = 1 << 1;
#[allow(unused)]
const INOUT_REP: u8 = 1 << 2;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_inout {
    pub eax: u32,
    pub port: u16,
    pub bytes: u8,
    pub flags: u8,

    /* fields used only by in-kernel emulation */
    addrsize: u8,
    segment: u8,
}

impl vm_inout {
    pub fn is_in(&self) -> bool {
        (self.flags & INOUT_IN) != 0
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_mmio {
    pub bytes: u8,
    pub read: u8,
    pub _pad: [u16; 3],
    pub gpa: u64,
    pub data: u64,
}

#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Copy, Clone)]
pub enum task_switch_reason {
        TSR_CALL,
        TSR_IRET,
        TSR_JMP,
        TSR_IDT_GATE,   // task gate in IDT
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_task_switch {
    tsssel: u16,                         // new TSS selector
    ext: c_int,                          // task switch due to external event
    errcode: c_uint,
    errcode_valid: c_int,                // push 'errcode' on the new stack
    reason: task_switch_reason,
    paging: vm_guest_paging,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct vm_exit {
    pub exitcode: c_int,
    pub inst_length: c_int,    // 0 means unknown
    pub rip: u64,
    pub u: vm_exit_payload,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_entry {
    pub cpuid: c_int,
    pub cmd: c_uint,
    pub exit_data: *mut c_void,
    pub u: vm_entry_payload,
}
impl vm_entry {
    pub fn new(cpuid: i32, cmd: vm_entry_cmds, exitp: *mut vm_exit, payload: vm_entry_payload) -> Self {
        vm_entry {
            cpuid,
            cmd: cmd as u32,
            exit_data: exitp as *mut c_void,
            u: payload,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union vm_exit_payload {
    pub inout: vm_inout,
    pub mmio: vm_mmio,
    pub paging: vm_exit_paging,
    pub inst_emul: vm_exit_inst_emul,
    pub vmx: vm_exit_vmx,
    pub svm: vm_exit_svm,
    pub msr: vm_exit_msr,
    pub spinup_ap: vm_exit_spinup_ap,
    pub hlt: vm_exit_hlt,
    pub ioapic_eoi: vm_exit_ioapic_eoi,
    pub suspended: vm_exit_suspended,
    pub task_switch: vm_task_switch,
    // sized to zero entire union
    empty: [u64; 6],
}

impl Default for vm_exit_payload {
    fn default() -> vm_exit_payload {
        vm_exit_payload { empty: [0u64; 6] }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union vm_entry_payload {
    pub inout: vm_inout,
    pub mmio: vm_mmio,
    // sized to zero entire union
    empty: [u64; 3],
}

impl Default for vm_entry_payload {
    fn default() -> vm_entry_payload {
        vm_entry_payload { empty: [0u64; 3] }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_paging {
    pub gpa: c_ulonglong,
    pub fault_type: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_inst_emul {
    pub inst: [u8; 15],
    pub num_valid: u8,
}

// VMX specific payload. Used when there is no "better"
// exitcode to represent the VM-exit.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_vmx {
    pub status: c_int,             // vmx inst status

    // 'exit_reason' and 'exit_qualification' are valid
    // only if 'status' is zero.
    pub exit_reason: c_uint,
    pub exit_qualification: c_ulonglong,

    // 'inst_error' and 'inst_type' are valid
    // only if 'status' is non-zero.
    pub inst_type: c_int,
    pub inst_error: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_svm {
    pub exitcode: c_ulonglong,
    pub exitinfo1: c_ulonglong,
    pub exitinfo2: c_ulonglong,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_msr {
    pub code: c_uint,      // ecx value
    pub wval: c_ulonglong,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_spinup_ap {
    pub vcpu: c_int,
    pub rip: c_ulonglong,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_hlt {
    pub rflags: c_ulonglong,
    pub intr_status: c_ulonglong,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_ioapic_eoi {
    pub vector: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vm_exit_suspended {
    pub how: vm_suspend_how,
}
