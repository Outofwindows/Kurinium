use std::arch::asm;

#[derive(Debug, Clone)]
pub struct CpuId {
    pub vendor: String,
    pub brand: String,
    pub cores: u32,
    pub threads: u32,
    pub features: CpuFeatures,
}

#[derive(Debug, Clone, Default)]
pub struct CpuFeatures {
    pub sse: bool,
    pub sse2: bool,
    pub sse3: bool,
    pub ssse3: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub aes: bool,
    pub hypervisor: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum TopologyLevel {
    Invalid = 0,
    Smt = 1,
    Core = 2,
    Module = 3,
    Tile = 4,
    Die = 5,
    Unknown = 255,
}

impl From<u8> for TopologyLevel {
    fn from(val: u8) -> Self {
        match val {
            0 => TopologyLevel::Invalid,
            1 => TopologyLevel::Smt,
            2 => TopologyLevel::Core,
            3 => TopologyLevel::Module,
            4 => TopologyLevel::Tile,
            5 => TopologyLevel::Die,
            _ => TopologyLevel::Unknown,
        }
    }
}

impl CpuId {
    pub fn get() -> Self {
        let (cores, threads) = Self::get_topology();
        
        Self {
            vendor: Self::get_vendor(),
            brand: Self::get_brand(),
            cores,
            threads,
            features: Self::get_features(),
        }
    }

    pub fn get_vendor() -> String {
        let (ebx, ecx, edx): (u32, u32, u32);

        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "mov {ebx_out:e}, ebx",
                "pop rbx",
                ebx_out = out(reg) ebx,
                inout("eax") 0u32 => _,
                out("ecx") ecx,
                out("edx") edx,
            );
        }

        let mut bytes = [0u8; 12];
        bytes[0..4].copy_from_slice(&ebx.to_le_bytes());
        bytes[4..8].copy_from_slice(&edx.to_le_bytes());
        bytes[8..12].copy_from_slice(&ecx.to_le_bytes());

        String::from_utf8_lossy(&bytes).to_string()
    }

    pub fn get_brand() -> String {
        let max_extended: u32;
        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "pop rbx",
                inout("eax") 0x80000000u32 => max_extended,
                out("ecx") _,
                out("edx") _,
            );
        }

        if max_extended < 0x80000004 {
            return "Unknown CPU".to_string();
        }

        let mut brand = [0u8; 48];

        for (i, leaf) in [0x80000002u32, 0x80000003, 0x80000004].iter().enumerate() {
            let (eax, ebx, ecx, edx): (u32, u32, u32, u32);

            unsafe {
                asm!(
                    "push rbx",
                    "cpuid",
                    "mov {ebx_out:e}, ebx",
                    "pop rbx",
                    ebx_out = out(reg) ebx,
                    inout("eax") *leaf => eax,
                    out("ecx") ecx,
                    out("edx") edx,
                );
            }

            let offset = i * 16;
            brand[offset..offset + 4].copy_from_slice(&eax.to_le_bytes());
            brand[offset + 4..offset + 8].copy_from_slice(&ebx.to_le_bytes());
            brand[offset + 8..offset + 12].copy_from_slice(&ecx.to_le_bytes());
            brand[offset + 12..offset + 16].copy_from_slice(&edx.to_le_bytes());
        }

        String::from_utf8_lossy(&brand)
            .trim_matches(char::from(0))
            .trim()
            .to_string()
    }

    fn get_topology() -> (u32, u32) {
        let max_leaf = Self::get_max_leaf();
        
        if max_leaf >= 0x1F {
            if let Some(result) = Self::get_topology_v2() {
                return result;
            }
        }
        
        if max_leaf >= 0x0B {
            if let Some(result) = Self::get_topology_x2apic() {
                return result;
            }
        }
        
        Self::get_topology_legacy()
    }

    fn get_max_leaf() -> u32 {
        let eax: u32;
        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "pop rbx",
                inout("eax") 0u32 => eax,
                out("ecx") _,
                out("edx") _,
            );
        }
        eax
    }

    fn get_topology_v2() -> Option<(u32, u32)> {
        Self::enumerate_topology(0x1F)
    }

    fn get_topology_x2apic() -> Option<(u32, u32)> {
        Self::enumerate_topology(0x0B)
    }

    fn enumerate_topology(leaf: u32) -> Option<(u32, u32)> {
        let mut smt_shift: u32 = 0;
        let mut core_shift: u32 = 0;
        let mut total_threads: u32 = 0;
        let mut found_smt = false;
        let mut found_core = false;

        for subleaf in 0..16u32 {
            let (eax, ebx, ecx): (u32, u32, u32);

            unsafe {
                asm!(
                    "push rbx",
                    "cpuid",
                    "mov {ebx_out:e}, ebx",
                    "pop rbx",
                    ebx_out = out(reg) ebx,
                    inout("eax") leaf => eax,
                    inout("ecx") subleaf => ecx,
                    out("edx") _,
                );
            }

            let level_type = TopologyLevel::from(((ecx >> 8) & 0xFF) as u8);
            let shift_bits = eax & 0x1F;
            let num_processors = ebx & 0xFFFF;

            if level_type == TopologyLevel::Invalid {
                break;
            }

            match level_type {
                TopologyLevel::Smt => {
                    smt_shift = shift_bits;
                    found_smt = true;
                }
                TopologyLevel::Core => {
                    core_shift = shift_bits;
                    total_threads = num_processors;
                    found_core = true;
                }
                _ => {}
            }
        }

        if !found_core {
            return None;
        }

        let threads_per_core = if found_smt && smt_shift > 0 {
            1u32 << smt_shift
        } else {
            1
        };

        let physical_cores = if total_threads > 0 && threads_per_core > 0 {
            total_threads / threads_per_core
        } else {
            if core_shift > smt_shift {
                1u32 << (core_shift - smt_shift)
            } else {
                1
            }
        };

        if physical_cores == 0 || total_threads == 0 {
            return None;
        }

        Some((physical_cores, total_threads))
    }

    fn get_topology_legacy() -> (u32, u32) {
        let vendor = Self::get_vendor();
        
        let threads = Self::get_thread_count_legacy();
        let cores = if vendor.contains("AMD") {
            Self::get_amd_core_count()
        } else {
            Self::get_intel_core_count_legacy()
        };

        (cores, threads)
    }

    fn get_thread_count_legacy() -> u32 {
        let ebx: u32;

        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "mov {ebx_out:e}, ebx",
                "pop rbx",
                ebx_out = out(reg) ebx,
                inout("eax") 1u32 => _,
                out("ecx") _,
                out("edx") _,
            );
        }

        let count = (ebx >> 16) & 0xFF;
        if count == 0 { 1 } else { count }
    }

    fn get_intel_core_count_legacy() -> u32 {
        let eax: u32;

        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "pop rbx",
                inout("eax") 4u32 => eax,
                inout("ecx") 0u32 => _,
                out("edx") _,
            );
        }

        let count = ((eax >> 26) & 0x3F) + 1;
        if count == 0 { 1 } else { count }
    }

    fn get_amd_core_count() -> u32 {
        let max_extended: u32;
        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "pop rbx",
                inout("eax") 0x80000000u32 => max_extended,
                out("ecx") _,
                out("edx") _,
            );
        }

        if max_extended < 0x80000008 {
            return 1;
        }

        let ecx: u32;
        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "pop rbx",
                inout("eax") 0x80000008u32 => _,
                out("ecx") ecx,
                out("edx") _,
            );
        }

        let count = (ecx & 0xFF) + 1;
        if count == 0 { 1 } else { count }
    }

    pub fn get_features() -> CpuFeatures {
        let (ecx, edx): (u32, u32);

        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "pop rbx",
                inout("eax") 1u32 => _,
                out("ecx") ecx,
                out("edx") edx,
            );
        }

        let avx2 = Self::check_avx2();

        CpuFeatures {
            sse:        (edx >> 25) & 1 == 1,
            sse2:       (edx >> 26) & 1 == 1,
            sse3:        ecx        & 1 == 1,
            ssse3:      (ecx >> 9)  & 1 == 1,
            sse4_1:     (ecx >> 19) & 1 == 1,
            sse4_2:     (ecx >> 20) & 1 == 1,
            avx:        (ecx >> 28) & 1 == 1,
            avx2,
            aes:        (ecx >> 25) & 1 == 1,
            hypervisor: (ecx >> 31) & 1 == 1,
        }
    }

    fn check_avx2() -> bool {
        let max_leaf = Self::get_max_leaf();
        if max_leaf < 7 {
            return false;
        }

        let ebx: u32;
        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "mov {ebx_out:e}, ebx",
                "pop rbx",
                ebx_out = out(reg) ebx,
                inout("eax") 7u32 => _,
                inout("ecx") 0u32 => _,
                out("edx") _,
            );
        }

        (ebx >> 5) & 1 == 1
    }
}

impl CpuFeatures {
    pub fn to_string_list(&self) -> Vec<&'static str> {
        let mut features = Vec::new();

        if self.sse    { features.push("SSE");    }
        if self.sse2   { features.push("SSE2");   }
        if self.sse3   { features.push("SSE3");   }
        if self.ssse3  { features.push("SSSE3");  }
        if self.sse4_1 { features.push("SSE4.1"); }
        if self.sse4_2 { features.push("SSE4.2"); }
        if self.avx    { features.push("AVX");    }
        if self.avx2   { features.push("AVX2");   }
        if self.aes    { features.push("AES-NI"); }

        features
    }
}