//! Bhyve virtual machine operations.

use libc::{ioctl, open, O_RDWR, c_void, sysconf, _SC_PAGESIZE, EINVAL, EFAULT};
use std::convert::TryFrom;
use std::ffi::{CString, CStr};
use std::fs::File;
use std::os::unix::io::{AsRawFd, FromRawFd};

pub use crate::include::vmm::{vm_cap_type, vm_reg_name};
use crate::include::vmm::{vm_suspend_how, vm_exitcode, x2apic_state, seg_desc};
use crate::include::vmm::{vm_entry, vm_entry_payload, vm_entry_cmds, vm_exit};
use crate::include::vmm_dev::*;
use crate::include::specialreg::{CR0_NE};
use crate::Error;

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

const MAX_BOOTROM_SIZE: usize = 16 * MB as usize;

// Size of the guard region before and after the virtual address space
// mapping the guest physical memory. This must be a multiple of the
// superpage size for performance reasons.
//const VM_MMAP_GUARD_SIZE: usize = 4 * MB as usize;

/// The VirtualMachine module handles Bhyve virtual machine operations.
/// It owns the filehandle for these operations.
pub struct VirtualMachine {
    vm: File,
    pub name: String,
    pub lowmem_limit: usize,
    pub memflags: i32,
}

impl VirtualMachine {
    /// Opens a filehandle to an existing virtual machine device by name, and
    /// returns a `Result`. If the open  operation fails, the `Result` unwraps
    /// as an `Error`. If it succeeds, the `Result` unwraps as an instance of
    /// `VirtualMachine`.

    pub fn new(name: &str) -> Result<VirtualMachine, Error> {
        let path = format!("/dev/vmm/{}", name);
        let c_path = match CString::new(path) {
            Ok(s) => s,
            Err(_) => return Err(Error::new(EINVAL))
        };
        let raw_fd = unsafe { open(c_path.as_ptr(), O_RDWR) };
        if raw_fd < 0 {
            return Err(Error::last());
        }
        let safe_handle = unsafe { File::from_raw_fd(raw_fd) };

        // Return value is safe because raw file descriptor result is checked
        // and ownership of File struct is consumed by VirtualMachine struct.
        Ok(VirtualMachine {
            vm: safe_handle,
            name: name.to_string(),
            lowmem_limit: 3 * GB as usize,
            memflags: 0,
        })
    }

    /// Map the memory segment identified by 'segid' into the guest address space
    /// at [gpa,gpa+len) with protection 'prot'.
    pub fn mmap_memseg(&self, gpa: u64, segid: i32, off: i64, len: usize, prot: i32) -> Result<bool, Error> {
        let mut flags = 0;
        if (self.memflags & VM_MEM_F_WIRED) != 0 {
            flags = VM_MEMMAP_F_WIRED;
        }

        let mem_data = vm_memmap {
            gpa: gpa,
            segid: segid,
            segoff: off,
            len: len,
            prot: prot,
            flags: flags,
        };

	// If this mapping already exists then don't create it again. This
	// is the common case for SYSMEM mappings created by bhyveload(8).
        match self.mmap_getnext(gpa) {
            Ok(exists) => if exists.gpa == mem_data.gpa {
                // A memory segment already exists at the same guest physical address
                // we are trying to create.
                if exists.segid == mem_data.segid && exists.segoff == mem_data.segoff &&
                   exists.prot == mem_data.prot && exists.flags == mem_data.flags {
                    // The existing memory segment is identical to the one we want to
                    // create, so do nothing, and return a success value.
                    return Ok(true);
                } else {
                    // The existing memory segment is not identical to the one we want
                    // to create, so return an error value.
                    return Err(Error::new(EFAULT));
                }
            }
            Err(_) => (), // The memory segment wasn't found, so we should create it
        };

        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_MMAP_MEMSEG, &mem_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Iterate over the guest address space. This function finds an address range
    /// that starts at an address >= 'gpa'.
    ///
    /// Returns Ok if the next address range was found and an Error otherwise.

    fn mmap_getnext(&self, gpa: u64) -> Result<vm_memmap, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut memseg_data = vm_memmap {
            gpa: gpa,
            ..Default::default()
        };

        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_MMAP_GETNEXT, &mut memseg_data) };
        if result == 0 {
            return Ok(memseg_data);
        } else {
            return Err(Error::last());
        }
    }

    /// Unmap the memory segment at the guest physical address range [gpa,gpa+len)
    pub fn munmap_memseg(&self, _gpa: u64, _len: usize) -> Result<bool, Error> {
        // leave unwired for now
        panic!("cannot munmap");
    }

    pub fn alloc_memseg(&self, segid: i32, len: usize, name: &str) -> Result<bool, Error> {
        let c_name = match CString::new(name) {
            Ok(s) => s,
            Err(_) => return Err(Error::new(EINVAL))
        };

        // If the memory segment has already been created then just return.
        // This is the usual case for the SYSMEM segment created by userspace
        // loaders like bhyveload(8).
        match self.get_memseg(segid) {
            Ok(exists) => if exists.len != 0 {
                // A memory segment already exists with the same segment ID as the one
                // we are trying to allocate.
                let r_name = unsafe { CStr::from_ptr(exists.name.as_ptr()) };
                let exists_name = r_name.to_owned();
                if exists.len == len && exists_name == c_name {
                    // The existing memory segment is identical to the one we want to
                    // allocate, so do nothing, and return a success value.
                    return Ok(true);
                } else {
                    // The existing memory segment is not identical to the one we want
                    // to allocate, so return an error value.
                    return Err(Error::new(EINVAL));
                }
            }
            Err(e) => return Err(e),
        };

        // Struct is allocated (and owned) by Rust
        let mut memseg_data = vm_memseg {
            segid: segid,
            len: len,
            ..Default::default()
        };

        let name_length = name.len();
        if name_length > 0 {
            // Don't copy the name if the string is empty (zero length)
            if name_length >= memseg_data.name.len() {
                // name is too long for vm_memseg struct
                return Err(Error::new(EINVAL));
            } else {
                // Copy each character from the CString to the char array
                for (to, from) in memseg_data.name.iter_mut().zip(c_name.as_bytes_with_nul()) {
                    *to = *from as i8;
                }
            }
        }

        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_ALLOC_MEMSEG, &memseg_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    fn get_memseg(&self, segid: i32) -> Result<vm_memseg, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut memseg_data = vm_memseg {
            segid: segid,
            ..Default::default()
        };

        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_GET_MEMSEG, &mut memseg_data) };
        if result == 0 {
            return Ok(memseg_data);
        } else {
            return Err(Error::last());
        }
    }

    fn add_devmem(&self, segid: i32, name: &str, base: u64, len: usize) -> Result<bool, Error> {
        self.alloc_memseg(segid, len, name)?;
        let mapoff = self.get_devmem_offset(segid)?;

//        let len2 = VM_MMAP_GUARD_SIZE + len + VM_MMAP_GUARD_SIZE;
//        let base: *mut u8 = unsafe {
//            libc::mmap(
//                null_mut(),
//                len2,
//                libc::PROT_NONE,
//                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
//                -1,
//                0,
//            ) as *mut u8
//        };

        // mmap the devmem region in the host address space
        let _ptr: *mut u8 = unsafe {
            libc::mmap(
                base as *mut c_void,
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_FIXED,
                self.vm.as_raw_fd(),
                mapoff,
            ) as *mut u8
        };
        return Ok(true);

    }

    pub fn add_guest_memory(&self, segid: i32, gpa: u64, base: u64, len: usize, readonly: bool) -> Result<bool, Error> {
        self.alloc_memseg(segid, len, "")?; // Unnamed memory regions, identified by segment id

        // Map the guest memory into the guest address space
	let prot = match readonly {
            true => libc::PROT_READ | libc::PROT_EXEC,
            false => libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
        };
	self.mmap_memseg(gpa, segid, 0, len, prot)?;

        // mmap into the process address space on the host
        let ptr = unsafe {
            libc::mmap(
                base as *mut c_void,
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_FIXED,
                self.vm.as_raw_fd(),
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(Error::new(EFAULT));
        }

        return Ok(true);

    }

    /// Gets the map offset for the device memory segment 'segid'.
    ///
    /// Returns Ok containing the offset if successful, and an Error otherwise.
    fn get_devmem_offset(&self, segid: i32) -> Result<i64, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut memseg_data = vm_devmem_offset {
            segid: segid,
            ..Default::default()
        };

        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_DEVMEM_GETOFFSET, &mut memseg_data) };
        if result == 0 {
            return Ok(memseg_data.offset);
        } else {
            return Err(Error::last());
        }
    }

    /// Sets up a memory segment for the bootrom
    ///
    /// Returns Ok if successful, and an Error otherwise.
    pub fn setup_bootrom(&self, base: u64, len: usize) -> Result<bool, Error> {

        let page_size: usize = unsafe { sysconf(_SC_PAGESIZE) as usize };
        // Limit bootrom size to 16MB so it doesn't encroach into reserved
        // MMIO space (e.g. APIC, HPET, MSI).
        if len > MAX_BOOTROM_SIZE || len < page_size {
            return Err(Error::new(EINVAL));
        }
        // Map the bootrom into the host address space
        self.add_devmem(MemSegId::VM_BOOTROM as i32, "bootrom", base, len)?;

        // Map the bootrom into the guest address space
	let prot = libc::PROT_READ | libc::PROT_EXEC;
	let gpa: u64 = (1 << 32) - len as u64;
	self.mmap_memseg(gpa, MemSegId::VM_BOOTROM as i32, 0, len, prot)?;

        Ok(true)
    }

    pub fn setup_lowmem(&self, base: u64, len: usize) -> Result<bool, Error> {
        if len > self.lowmem_limit {
            return Err(Error::new(EINVAL));
        }

	let gpa: u64 = 0;
        let readonly = false;
        // Map the guest memory into the host address space
        self.add_guest_memory(MemSegId::VM_LOWMEM as i32, gpa, base, len, readonly)?;

        Ok(true)
    }

    pub fn setup_highmem(&self, base: u64, len: usize) -> Result<bool, Error> {
	let gpa: u64 = 4 * GB;
        let readonly = false;
        // Map the guest memory into the host address space
        self.add_guest_memory(MemSegId::VM_HIGHMEM as i32, gpa, base, len, readonly)?;

        Ok(true)
    }

    /// Set the base, limit, and access values of a descriptor register on the VCPU
    pub fn set_desc(&self, vcpu_id: i32, reg: vm_reg_name, base: u64, limit: u32, access: u32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let seg_data = vm_seg_desc {
            cpuid: vcpu_id,
            regnum: reg as i32,
            desc: seg_desc {base: base, limit: limit, access: access},
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SET_SEGMENT_DESCRIPTOR, &seg_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Get the base, limit, and access values of a descriptor register on the VCPU
    pub fn get_desc(&self, vcpu_id: i32, reg: vm_reg_name) -> Result<(u64, u32, u32), Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut seg_data = vm_seg_desc {
            cpuid: vcpu_id,
            regnum: reg as i32,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_GET_SEGMENT_DESCRIPTOR, &mut seg_data) };
        if result == 0 {
            return Ok((seg_data.desc.base, seg_data.desc.limit, seg_data.desc.access));
        } else {
            return Err(Error::last());
        }
    }

    /// Set the value of a single register on the VCPU
    pub fn set_register(&self, vcpu_id: i32, reg: vm_reg_name, val: u64) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let reg_data = vm_register {
            cpuid: vcpu_id,
            regnum: reg as i32,
            regval: val,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SET_REGISTER, &reg_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Get the value of a single register on the VCPU
    pub fn get_register(&self, vcpu_id: i32, reg: vm_reg_name) -> Result<u64, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut reg_data = vm_register {
            cpuid: vcpu_id,
            regnum: reg as i32,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_GET_REGISTER, &mut reg_data) };
        if result == 0 {
            return Ok(reg_data.regval);
        } else {
            return Err(Error::last());
        }
    }

    pub fn rtc_write(&self, offset: i32, value: u8) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let rtc_data = vm_rtc_data {
            offset: offset,
            value: value,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_RTC_WRITE, &rtc_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    pub fn rtc_read(&self, offset: i32) -> Result<u8, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut rtc_data = vm_rtc_data {
            offset: offset,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_RTC_READ, &mut rtc_data) };
        if result == 0 {
            return Ok(rtc_data.value);
        } else {
            return Err(Error::last());
        }
    }

    pub fn rtc_settime(&self, secs: i64) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let rtc_data = vm_rtc_time {
            secs: secs,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_RTC_SETTIME, &rtc_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    pub fn rtc_gettime(&self) -> Result<i64, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut rtc_data = vm_rtc_time::default();
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_RTC_GETTIME, &mut rtc_data) };
        if result == 0 {
            return Ok(rtc_data.secs);
        } else {
            return Err(Error::last());
        }
    }

    /// Sets basic attributes of CPUs on the VirtualMachine: sockets, cores,
    /// and threads.
    pub fn set_topology(&self, sockets: u16, cores: u16, threads: u16) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let top_data = vm_cpu_topology {
            sockets: sockets,
            cores: cores,
            threads: threads,
            maxcpus: 0, // any other value is invalid
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SET_TOPOLOGY, &top_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Gets current settings for CPUs on the VirtualMachine: sockets, cores,
    /// threads, and maximum number of CPUs.
    pub fn get_topology(&self) -> Result<(u16, u16, u16, u16), Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut top = vm_cpu_topology::default();
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_GET_TOPOLOGY, &mut top) };
        if result == 0 {
            return Ok((top.sockets, top.cores, top.threads, top.maxcpus));
        } else {
            return Err(Error::last());
        }
    }

    /// Gets current stats for a CPUs on the VirtualMachine.
    pub fn get_stats(&self, vcpu_id: i32) -> Result<i32, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut stats_data = vm_stats {
            cpuid: vcpu_id,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_STATS_IOC, &mut stats_data) };
        if result == 0 {
            return Ok(stats_data.num_entries);
        } else {
            return Err(Error::last());
        }
    }

    /// Activates a Virtual CPU on the VirtualMachine.
    pub fn activate_vcpu(&self, vcpu_id: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let cpu_data = vm_activate_cpu { vcpuid: vcpu_id };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_ACTIVATE_CPU, &cpu_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    pub fn set_x2apic_state(&self, vcpu_id: i32, enable: bool) -> Result<bool, Error> {
        let state = match enable {
            true => x2apic_state::X2APIC_ENABLED,
            false => x2apic_state::X2APIC_DISABLED
        };

        // Struct is allocated (and owned) by Rust
        let x2apic_data = vm_x2apic {
            cpuid: vcpu_id,
            state: state,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SET_X2APIC_STATE, &x2apic_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    pub fn get_x2apic_state(&self, vcpu_id: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut x2apic_data = vm_x2apic {
            cpuid: vcpu_id,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_GET_X2APIC_STATE, &mut x2apic_data) };
        if result == 0 {
            match x2apic_data.state {
                x2apic_state::X2APIC_ENABLED => return Ok(true),
                x2apic_state::X2APIC_DISABLED => return Ok(false),
                x2apic_state::X2APIC_STATE_LAST => return Err(Error::new(EINVAL)),
            }
        } else {
            return Err(Error::last());
        }
    }

    /// From Intel Vol 3a:
    /// Table 9-1. IA-32 Processor States Following Power-up, Reset or INIT
    pub fn vcpu_reset(&self, vcpu_id: i32) -> Result<bool, Error> {
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RFLAGS, 0x2)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RIP, 0xfff0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_CR0, CR0_NE)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_CR3, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_CR4, 0)?;

        // CS: present, r/w, accessed, 16-bit, byte granularity, usable
	let cs_base = 0xffff0000;
	let cs_limit = 0xffff;
	let cs_access = 0x0093;
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_CS, cs_base, cs_limit, cs_access)?;

        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_CS, 0xf000)?;


        // SS,DS,ES,FS,GS: present, r/w, accessed, 16-bit, byte granularity
	let desc_base = 0;
	let desc_limit = 0xffff;
	let desc_access = 0x0093;
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_SS, desc_base, desc_limit, desc_access)?;
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_DS, desc_base, desc_limit, desc_access)?;
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_ES, desc_base, desc_limit, desc_access)?;
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_FS, desc_base, desc_limit, desc_access)?;
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_GS, desc_base, desc_limit, desc_access)?;

        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_SS, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_DS, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_ES, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_FS, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_GS, 0)?;

        // General purpose registers
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RAX, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RBX, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RCX, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RDX, 0xf00)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RSI, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RDI, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RBP, 0)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_RSP, 0)?;


        // GDTR, IDTR
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_GDTR, 0, 0xffff, 0)?;
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_IDTR, 0, 0xffff, 0)?;

        // TR
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_TR, 0, 0, 0x0000008b)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_TR, 0)?;

        // LDTR
        self.set_desc(vcpu_id, vm_reg_name::VM_REG_GUEST_LDTR, 0, 0xffff, 0x00000082)?;
        self.set_register(vcpu_id, vm_reg_name::VM_REG_GUEST_LDTR, 0)?;

        Ok(true)
    }

    /// Suspends a Virtual CPU on the VirtualMachine.
    pub fn suspend_vcpu(&self, vcpu_id: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let cpu_data = vm_activate_cpu { vcpuid: vcpu_id };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SUSPEND_CPU, &cpu_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Resumes a Virtual CPU on the VirtualMachine.
    pub fn resume_vcpu(&self, vcpu_id: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let cpu_data = vm_activate_cpu { vcpuid: vcpu_id };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_RESUME_CPU, &cpu_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Runs the VirtualMachine, and returns an exit reason.
    pub fn run(&self, vcpu_id: i32, entry: VmEntry) -> Result<VmExit, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let (result, exit_data) = unsafe {
            let mut vme = vm_exit::default();
            let entry_payload = vm_entry_payload::default();

            let entry = vm_entry::new(vcpu_id, vm_entry_cmds::VEC_DEFAULT, &mut vme, entry_payload);
            let res = ioctl(self.vm.as_raw_fd(), VM_RUN, &entry);
            (res, vme)
        };

        if result != 0 {
            return Err(Error::last());
        }

        let code = match vm_exitcode::try_from(exit_data.exitcode) {
            Err(_) => {
                return Err(Error::new(libc::ERANGE));
            },
            Ok(code) => code,
        };

        // let rip = exit_data.vm_exit.rip;
        // println!("RIP after run is {}", rip);
        // let cid = exit_data.cpuid;
        // println!("VCPU ID is {}", cid);

        match code {
            vm_exitcode::VM_EXITCODE_INOUT => {
                // Safe because the exit code told us which union field to use.
                let io = unsafe { exit_data.u.inout };

                if io.is_in() {
                    Ok(VmExit::IoIn(io.port, io.bytes))
                } else {
                    Ok(VmExit::IoOut(io.port, io.bytes, io.eax))
                }
            }
            vm_exitcode::VM_EXITCODE_MMIO => {
                // Safe because the exit code told us which union field to use.
                let mmio = unsafe { exit_data.u.mmio };

                if mmio.read == 0 {
                    Ok(VmExit::MmioRead(mmio.gpa, mmio.bytes))
                } else {
                    Ok(VmExit::MmioWrite(mmio.gpa, mmio.bytes, mmio.data))
                }
            }
            vm_exitcode::VM_EXITCODE_VMX => {
                let status = unsafe { exit_data.u.vmx.status };
                let reason = unsafe { exit_data.u.vmx.exit_reason };
                let qual = unsafe { exit_data.u.vmx.exit_qualification };
                let inst_type = unsafe { exit_data.u.vmx.inst_type };
                let inst_error = unsafe { exit_data.u.vmx.inst_error };
                Ok(VmExit::Vmx(status, reason, qual, inst_type, inst_error))
            }
            vm_exitcode::VM_EXITCODE_BOGUS => {
                Ok(VmExit::Bogus)
            }
            vm_exitcode::VM_EXITCODE_RDMSR => {
                Ok(VmExit::RdMsr)
            }
            vm_exitcode::VM_EXITCODE_WRMSR => {
                Ok(VmExit::WrMsr)
            }
            vm_exitcode::VM_EXITCODE_HLT => {
                Ok(VmExit::Halt)
            }
            vm_exitcode::VM_EXITCODE_MTRAP => {
                Ok(VmExit::Mtrap)
            }
            vm_exitcode::VM_EXITCODE_PAUSE => {
                Ok(VmExit::Pause)
            }
            vm_exitcode::VM_EXITCODE_PAGING => {
                Ok(VmExit::Paging)
            }
            vm_exitcode::VM_EXITCODE_INST_EMUL => {
                Ok(VmExit::InstEmul)
            }
            vm_exitcode::VM_EXITCODE_SPINUP_AP => {
                Ok(VmExit::SpinupAp)
            }
            vm_exitcode::VM_EXITCODE_RUNBLOCK => {
                Ok(VmExit::RunBlock)
            }
            vm_exitcode::VM_EXITCODE_IOAPIC_EOI => {
                let ioapic = unsafe { exit_data.u.ioapic_eoi };
                Ok(VmExit::IoapicEoi(ioapic.vector))
            }
            vm_exitcode::VM_EXITCODE_SUSPENDED => {
                Ok(VmExit::Suspended)
            }
            vm_exitcode::VM_EXITCODE_TASK_SWITCH => {
                Ok(VmExit::TaskSwitch)
            }
            vm_exitcode::VM_EXITCODE_MONITOR => {
                Ok(VmExit::Monitor)
            }
            vm_exitcode::VM_EXITCODE_MWAIT => {
                Ok(VmExit::Mwait)
            }
            vm_exitcode::VM_EXITCODE_SVM => {
                let svm = unsafe { exit_data.u.svm };
                Ok(VmExit::Svm(svm.exitcode, svm.exitinfo1, svm.exitinfo2))
            }
            vm_exitcode::VM_EXITCODE_REQIDLE => {
                Ok(VmExit::ReqIdle)
            }
            vm_exitcode::VM_EXITCODE_DEBUG => {
                Ok(VmExit::Debug)
            }
            vm_exitcode::VM_EXITCODE_VMINSN => {
                Ok(VmExit::VmInsn)
            }
            vm_exitcode::VM_EXITCODE_HT => {
                Ok(VmExit::Ht)
            }
            _ => {
                panic!("unexpected exit {:?}", code);
            }
        }
    }

    /// Resets the VirtualMachine.
    pub fn reset(&self) -> Result<i32, Error> {
        let suspend_data = vm_suspend { how: vm_suspend_how::VM_SUSPEND_RESET };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SUSPEND, &suspend_data) };
        if result == 0 {
            return Ok(result);
        } else {
            return Err(Error::last());
        }
    }

    /// Halts the VirtualMachine.
    pub fn halt(&self) -> Result<i32, Error> {
        let suspend_data = vm_suspend { how: vm_suspend_how::VM_SUSPEND_HALT };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SUSPEND, &suspend_data) };
        if result == 0 {
            return Ok(result);
        } else {
            return Err(Error::last());
        }
    }

    /// Suspends the VirtualMachine with power off.
    pub fn poweroff(&self) -> Result<i32, Error> {
        let suspend_data = vm_suspend { how: vm_suspend_how::VM_SUSPEND_POWEROFF };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SUSPEND, &suspend_data) };
        if result == 0 {
            return Ok(result);
        } else {
            return Err(Error::last());
        }
    }

    /// Suspends the VirtualMachine with triple fault.
    pub fn triplefault(&self) -> Result<i32, Error> {
        let suspend_data = vm_suspend { how: vm_suspend_how::VM_SUSPEND_TRIPLEFAULT };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SUSPEND, &suspend_data) };
        if result == 0 {
            return Ok(result);
        } else {
            return Err(Error::last());
        }
    }

    /// Reinitializes the VirtualMachine.
    pub fn reinit(&self) -> Result<i32, Error> {
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_REINIT) };
        if result == 0 {
            return Ok(result);
        } else {
            return Err(Error::last());
        }
    }

    /// Get the value of an optional capability on the VCPU
    pub fn get_capability(&self, vcpu_id: i32, cap: vm_cap_type) -> Result<i32, Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut cap_data = vm_capability {
            cpuid: vcpu_id,
            captype: cap,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_GET_CAPABILITY, &mut cap_data) };
        if result == 0 {
            return Ok(cap_data.capval);
        } else {
            return Err(Error::last());
        }
    }

    /// Set the value of an optional capability on the VCPU
    pub fn set_capability(&self, vcpu_id: i32, cap: vm_cap_type, val: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let cap_data = vm_capability {
            cpuid: vcpu_id,
            captype: cap,
            capval: val,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SET_CAPABILITY, &cap_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Set interrupt info on the VCPU
    pub fn set_intinfo(&self, vcpu_id: i32, info1: u64) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let intinfo_data = vm_intinfo {
            vcpuid: vcpu_id,
            info1: info1,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_SET_INTINFO, &intinfo_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Get the interrupt info on the VCPU
    pub fn get_intinfo(&self, vcpu_id: i32) -> Result<(u64, u64), Error> {
        // Struct is allocated (and owned) by Rust, but modified by C
        let mut intinfo_data = vm_intinfo {
            vcpuid: vcpu_id,
            ..Default::default()
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_GET_INTINFO, &mut intinfo_data) };
        if result == 0 {
            return Ok((intinfo_data.info1, intinfo_data.info2));
        } else {
            return Err(Error::last());
        }
    }

    /// Inject an exception on the VCPU
    pub fn inject_exception(&self, vcpu_id: i32, vector: i32, valid: i32, errcode: u32, restart: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let exc_data = vm_exception {
            cpuid: vcpu_id,
            vector: vector,
            error_code: errcode,
            error_code_valid: valid,
            restart_instruction: restart,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_INJECT_EXCEPTION, &exc_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Inject non-maskable interrupt (NMI) on the VCPU
    pub fn inject_nmi(&self, vcpu_id: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let nmi_data = vm_nmi {
            cpuid: vcpu_id,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_INJECT_NMI, &nmi_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Signal to the Local Advanced Programmable Interrupt Controller (LAPIC)
    /// that an interrupt request (IRQ) at 'vector' needs to be sent to the VCPU
    /// identified by 'vcpu_id'. The state of the interrupt request is recorded in
    /// the LAPIC interrupt request register (IRR).
    pub fn lapic_irq(&self, vcpu_id: i32, vector: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let irq_data = vm_lapic_irq {
            cpuid: vcpu_id,
            vector: vector,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_LAPIC_IRQ, &irq_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Trigger an interrupt request (IRQ) according to the local vector table
    /// (LVT) on the Local Advanced Programmable Interrupt Controller (LAPIC)
    /// for the VCPU identified by 'vcpu_id'. The 'vcpu_id' can be set to -1 to
    /// trigger the interrupt on all VCPUs.
    pub fn lapic_local_irq(&self, vcpu_id: i32, vector: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let irq_data = vm_lapic_irq {
            cpuid: vcpu_id,
            vector: vector,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_LAPIC_LOCAL_IRQ, &irq_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Signal an interrupt using Message Signaled Interrupts (MSI)
    pub fn lapic_msi(&self, addr: u64, msg: u64) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let msi_data = vm_lapic_msi {
            msg: msg,
            addr: addr,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_LAPIC_MSI, &msi_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Set the I/O APIC pin state for an interrupt request (IRQ) on the VM to true.
    pub fn ioapic_assert_irq(&self, irq: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let irq_data = vm_ioapic_irq {
            irq: irq,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_IOAPIC_ASSERT_IRQ, &irq_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Set the I/O APIC pin state for an interrupt request (IRQ) on the VM to false.
    pub fn ioapic_deassert_irq(&self, irq: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let irq_data = vm_ioapic_irq {
            irq: irq,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_IOAPIC_DEASSERT_IRQ, &irq_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Set the I/O APIC pin state for an interrupt request (IRQ) on the VM to
    /// true and then false (a "pulse").
    pub fn ioapic_pulse_irq(&self, irq: i32) -> Result<bool, Error> {
        // Struct is allocated (and owned) by Rust
        let irq_data = vm_ioapic_irq {
            irq: irq,
        };
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_IOAPIC_PULSE_IRQ, &irq_data) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }

    /// Get the I/O APIC pincount for the VM
    pub fn ioapic_pincount(&self) -> Result<i32, Error> {
        // Integer is allocated (and owned) by Rust, but modified by C
        let mut pincount: i32 = 0;
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_IOAPIC_PINCOUNT, &mut pincount) };
        if result == 0 {
            return Ok(pincount);
        } else {
            return Err(Error::last());
        }
    }

    /// Restart the current instruction on the VCPU
    pub fn restart_instruction(&self, vcpu_id: i32) -> Result<bool, Error> {
        // Integer is allocated (and owned) by Rust
        let result = unsafe { ioctl(self.vm.as_raw_fd(), VM_RESTART_INSTRUCTION, &vcpu_id) };
        if result == 0 {
            return Ok(true);
        } else {
            return Err(Error::last());
        }
    }
}

// Different styles of mapping the memory assigned to a VM into the address
// space of the controlling process.
#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Debug, Copy, Clone)]
enum vm_mmap_style {
	VM_MMAP_NONE,		/* no mapping */
	VM_MMAP_ALL,		/* fully and statically mapped */
	VM_MMAP_SPARSE,		/* mappings created on-demand */
}

// 'flags' value passed to 'vm_set_memflags()'.
//const VM_MEM_F_INCORE: i32 = 0x01;    // include guest memory in core file
const VM_MEM_F_WIRED: i32 = 0x02;	// guest memory is wired

/// Identifiers for memory segments, both system memory and devmem segments.
#[repr(C)]
#[allow(non_camel_case_types, unused)]
#[derive(Debug, Copy, Clone)]
pub enum MemSegId{
        VM_LOWMEM = 0,
        VM_HIGHMEM = 1,
        VM_BOOTROM = 2,
        VM_FRAMEBUFFER = 3,
}

/// Reasons for virtual machine exits.
///
/// The exit reasons are mapped to the `VM_EXIT_*` defines in `machine/vmm.h`.
///
#[derive(Debug)]
pub enum VmExit {
    IoIn(u16 /* port */, u8 /* bytes */),
    IoOut(u16 /* port */, u8 /* bytes */, u32 /* value */),
    MmioRead(u64 /* gpa */, u8 /* bytes */),
    MmioWrite(u64 /* gpa */, u8 /* bytes */, u64 /* value */),
    Vmx(i32 /* status */, u32 /* exit reason */, u64 /* exit qualification */, i32 /* instruction type */, i32 /* instruction error */),
    Bogus,
    RdMsr,
    WrMsr,
    Halt,
    Mtrap,
    Pause,
    Paging,
    InstEmul,
    SpinupAp,
    RunBlock,
    IoapicEoi(i32 /* vector */),
    Suspended,
    TaskSwitch,
    Monitor,
    Mwait,
    Svm(u64 /* exitcode */, u64 /* exitinfo1 */, u64 /* exitinfo2 */),
    ReqIdle,
    Debug,
    VmInsn,
    Ht,
}

#[derive(Copy, Clone, Debug)]
pub enum VmEntry {
    Normal,
    CompleteIoIn(u16 /* port */, u8 /* bytes */, u32 /* eax */),
    CompleteIoOut(u16 /* port */, u8 /* bytes */),
    CompleteMmioRead(u64 /* gpa */, u8 /* bytes */, u64 /* data */),
    CompleteMmioWrite(u64 /* gpa */, u8 /* bytes */),
}
