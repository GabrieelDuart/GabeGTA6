//! `victory gpu`: what the host GPU and D3D12 runtime offer, before a title
//! is started. On Windows 10 the system's runtime stops below shader model
//! 6.6 and refuses directly indexed heaps; the DirectX 12 Agility SDK beside
//! `victory.exe` (docs/gtx1060.md) lifts that, and this shows whether it
//! was loaded.

use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, IDXGIAdapter1, IDXGIFactory4};
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use windows::core::{Interface, w};

/// Lines describing the default adapter's D3D12 device, and what the
/// translation needs of it.
pub fn lines() -> Vec<String> {
    let mut out = Vec::new();
    let mut device: Option<ID3D12Device> = None;
    // SAFETY: plain creation on the default adapter.
    if let Err(e) = unsafe { D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device) } {
        out.push(format!("no D3D12 device: {e}"));
        return out;
    }
    let Some(device) = device else {
        out.push("no D3D12 device".into());
        return out;
    };
    // SAFETY: plain queries.
    unsafe {
        // The runtime that answered: the Agility SDK's D3D12Core.dll or the system's.
        if let Ok(core) = GetModuleHandleW(w!("D3D12Core.dll")) {
            let mut path = [0u16; 512];
            let n = GetModuleFileNameW(Some(core), &mut path) as usize;
            out.push(format!("D3D12 runtime: {}", String::from_utf16_lossy(&path[..n])));
        }
        if let Ok(factory) = CreateDXGIFactory2::<IDXGIFactory4>(DXGI_CREATE_FACTORY_FLAGS(0))
            && let Ok(adapter) = factory.EnumAdapterByLuid::<IDXGIAdapter1>(device.GetAdapterLuid())
            && let Ok(d) = adapter.GetDesc1()
        {
            let name = String::from_utf16_lossy(&d.Description).trim_end_matches('\0').to_owned();
            out.push(format!(
                "adapter: {name} (vendor {:#06x}, {} MB of video memory)",
                d.VendorId,
                d.DedicatedVideoMemory >> 20
            ));
        }
        let levels = [
            D3D_FEATURE_LEVEL_12_2,
            D3D_FEATURE_LEVEL_12_1,
            D3D_FEATURE_LEVEL_12_0,
            D3D_FEATURE_LEVEL_11_1,
            D3D_FEATURE_LEVEL_11_0,
        ];
        let mut fl = D3D12_FEATURE_DATA_FEATURE_LEVELS {
            NumFeatureLevels: levels.len() as u32,
            pFeatureLevelsRequested: levels.as_ptr(),
            ..Default::default()
        };
        if device
            .CheckFeatureSupport(D3D12_FEATURE_FEATURE_LEVELS, (&raw mut fl).cast(), size_of_val(&fl) as u32)
            .is_ok()
        {
            let l = fl.MaxSupportedFeatureLevel.0;
            out.push(format!("feature level: {}_{}", l >> 12, (l >> 8) & 0xf));
        }
        let mut sm = 0;
        for model in [0x69, 0x68, 0x67, 0x66, 0x65, 0x64, 0x63, 0x62, 0x61, 0x60] {
            let mut m = D3D12_FEATURE_DATA_SHADER_MODEL { HighestShaderModel: D3D_SHADER_MODEL(model) };
            if device
                .CheckFeatureSupport(D3D12_FEATURE_SHADER_MODEL, (&raw mut m).cast(), size_of_val(&m) as u32)
                .is_ok()
            {
                sm = m.HighestShaderModel.0;
                break;
            }
        }
        out.push(format!(
            "shader model: {}.{}{}",
            sm >> 4,
            sm & 0xf,
            if sm >= 0x66 { "" } else { "  <- below 6.6: the title's shaders cannot run (Agility SDK? driver?)" }
        ));
        let mut o = D3D12_FEATURE_DATA_D3D12_OPTIONS::default();
        if device
            .CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS, (&raw mut o).cast(), size_of_val(&o) as u32)
            .is_ok()
        {
            out.push(format!("resource binding tier: {}", o.ResourceBindingTier.0));
        }
        let mut o1 = D3D12_FEATURE_DATA_D3D12_OPTIONS1::default();
        if device
            .CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS1, (&raw mut o1).cast(), size_of_val(&o1) as u32)
            .is_ok()
        {
            out.push(format!("wave lanes: {} to {}", o1.WaveLaneCountMin, o1.WaveLaneCountMax));
        }
        let mut o7 = D3D12_FEATURE_DATA_D3D12_OPTIONS7::default();
        let mesh = device
            .CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS7, (&raw mut o7).cast(), size_of_val(&o7) as u32)
            .is_ok()
            && o7.MeshShaderTier != D3D12_MESH_SHADER_TIER_NOT_SUPPORTED;
        out.push(format!(
            "mesh shaders: {}",
            if mesh { "yes" } else { "no (mesh programs run as compute programs and a draw)" }
        ));
        // A root signature with directly indexed heaps, as every translated shader has.
        let desc = D3D12_VERSIONED_ROOT_SIGNATURE_DESC {
            Version: D3D_ROOT_SIGNATURE_VERSION_1_1,
            Anonymous: D3D12_VERSIONED_ROOT_SIGNATURE_DESC_0 {
                Desc_1_1: D3D12_ROOT_SIGNATURE_DESC1 {
                    Flags: D3D12_ROOT_SIGNATURE_FLAG_CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED
                        | D3D12_ROOT_SIGNATURE_FLAG_SAMPLER_HEAP_DIRECTLY_INDEXED,
                    ..Default::default()
                },
            },
        };
        let mut blob = None;
        let ok = D3D12SerializeVersionedRootSignature(&desc, &mut blob, None).is_ok()
            && blob.is_some_and(|b: ID3DBlob| {
                let bytes = std::slice::from_raw_parts(b.GetBufferPointer().cast::<u8>(), b.GetBufferSize());
                device.CreateRootSignature::<ID3D12RootSignature>(0, bytes).is_ok()
            });
        out.push(format!(
            "directly indexed heaps: {}",
            if ok { "yes" } else { "no  <- every pipeline would be refused" }
        ));
        let _ = device.cast::<ID3D12Device>();
    }
    out
}

/// The default adapter's widest wave and whether it runs mesh shaders, for
/// translating ahead of a run (`victory shaders`, `translate --warm`) as the
/// run will (the translation, and its cache key, depend on both).
pub fn caps() -> Option<(u32, bool)> {
    let mut device: Option<ID3D12Device> = None;
    // SAFETY: plain creation on the default adapter, then plain queries.
    unsafe {
        D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device).ok()?;
        let device = device?;
        let mut o1 = D3D12_FEATURE_DATA_D3D12_OPTIONS1::default();
        device
            .CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS1, (&raw mut o1).cast(), size_of_val(&o1) as u32)
            .ok()?;
        let mut o7 = D3D12_FEATURE_DATA_D3D12_OPTIONS7::default();
        let mesh = device
            .CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS7, (&raw mut o7).cast(), size_of_val(&o7) as u32)
            .is_ok()
            && o7.MeshShaderTier != D3D12_MESH_SHADER_TIER_NOT_SUPPORTED;
        Some((o1.WaveLaneCountMax, mesh))
    }
}

/// Sets the translator up as a run on this PC would ([`caps`]): its wave
/// width unless `wave` names one, and mesh programs as compute programs
/// where there are no mesh shaders (`VICTORY_MESH_AS_COMPUTE` overrides).
/// What it found, for a line of the command's output.
pub fn translate_as_host(wave: Option<u32>) -> String {
    let caps = caps();
    if wave.is_none()
        && let Some((lanes, _)) = caps
    {
        victory_shader::emit::set_host_max_wave(lanes);
    }
    let mesh_as_compute = std::env::var("VICTORY_MESH_AS_COMPUTE")
        .map_or_else(|_| caps.is_some_and(|(_, mesh)| !mesh), |v| v == "1");
    victory_shader::emit::set_mesh_as_compute(mesh_as_compute);
    match caps {
        Some((lanes, mesh)) => format!(
            "translating as for this PC's GPU: waves of {}, mesh shaders {}{}",
            wave.unwrap_or(lanes),
            if mesh { "yes" } else { "no" },
            if mesh_as_compute { " (mesh programs as compute programs)" } else { "" }
        ),
        None => "no D3D12 device: translating with the defaults".into(),
    }
}
