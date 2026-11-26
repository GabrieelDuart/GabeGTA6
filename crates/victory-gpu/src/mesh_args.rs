//! The arguments of an indirect mesh dispatch on a host without mesh shaders
//! (`crate::pipeline::host::MeshAsCompute`): from the title's clamped mesh
//! record (groups in x), a compute dispatch of as many groups as the buffer
//! for their records holds, and a draw of three vertices for every triangle
//! those groups may make. A shader of ours writes both into a small buffer
//! of ours, which the two indirect executions then read; the CPU never sees
//! the group count.

use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::ID3DBlob;
use windows::Win32::Graphics::Direct3D12::*;
use windows::core::s;

use crate::host::{HostList, host, transition};

/// Where the dispatch record is in the buffer; the draw record follows it.
pub(crate) const DISPATCH_AT: u64 = 0;
pub(crate) const DRAW_AT: u64 = 16;

const SOURCE: &str = r#"
#define RS "RootConstants(num32BitConstants=2, b0), SRV(t0), UAV(u0)"
cbuffer Mesh : register(b0) {
    uint max_primitives;
    // Groups whose records fit the buffer for them.
    uint capacity;
};
ByteAddressBuffer record : register(t0);
RWByteAddressBuffer args : register(u0);
[RootSignature(RS)]
[numthreads(1, 1, 1)]
void main() {
    uint groups = min(record.Load(0), capacity);
    args.Store3(0, uint3(groups, 1, 1));
    args.Store4(16, uint4(groups * max_primitives * 3u, 1, 0, 0));
}
"#;

struct Made {
    root: ID3D12RootSignature,
    pso: ID3D12PipelineState,
    args: ID3D12Resource,
}

// SAFETY: D3D12 objects are free-threaded.
unsafe impl Send for Made {}
// SAFETY: as above.
unsafe impl Sync for Made {}

fn made() -> Option<&'static Made> {
    static MADE: std::sync::OnceLock<Option<Made>> = std::sync::OnceLock::new();
    MADE.get_or_init(|| match make() {
        Ok(m) => Some(m),
        Err(e) => {
            tracing::warn!(error = %e, "no arguments for indirect mesh dispatches run as compute programs: they are dropped");
            None
        }
    })
    .as_ref()
}

fn make() -> windows::core::Result<Made> {
    let host = host().ok_or_else(windows::core::Error::empty)?;
    let device = &host.device;
    let mut code: Option<ID3DBlob> = None;
    let mut errors: Option<ID3DBlob> = None;
    // SAFETY: the source and names live through the call; out pointers ours.
    let compiled = unsafe {
        D3DCompile(
            SOURCE.as_ptr().cast(),
            SOURCE.len(),
            s!("mesh_args"),
            None,
            None,
            s!("main"),
            s!("cs_5_1"),
            0,
            0,
            &mut code,
            Some(&mut errors),
        )
    };
    if let Err(e) = compiled {
        if let Some(errors) = errors {
            // SAFETY: the blob's own bytes.
            let text = unsafe {
                std::slice::from_raw_parts(errors.GetBufferPointer().cast::<u8>(), errors.GetBufferSize())
            };
            tracing::warn!("mesh arguments: {}", String::from_utf8_lossy(text));
        }
        return Err(e);
    }
    let code = code.ok_or_else(windows::core::Error::empty)?;
    let args = host.uav_buffer(32).ok_or_else(windows::core::Error::empty)?;
    // SAFETY: the blob's own bytes; the root signature is inside the shader.
    unsafe {
        let bytes = std::slice::from_raw_parts(code.GetBufferPointer().cast::<u8>(), code.GetBufferSize());
        let root: ID3D12RootSignature = device.CreateRootSignature(0, bytes)?;
        let pso = device.CreateComputePipelineState(&D3D12_COMPUTE_PIPELINE_STATE_DESC {
            pRootSignature: std::mem::transmute_copy(&root),
            CS: D3D12_SHADER_BYTECODE {
                pShaderBytecode: bytes.as_ptr().cast(),
                BytecodeLength: bytes.len(),
            },
            ..Default::default()
        })?;
        crate::host::name(&pso, || "mesh arguments".into());
        Ok(Made { root, pso, args })
    }
}

/// Writes the arguments for the mesh record at `record` (a GPU address in a
/// buffer the list has readable by shaders) and leaves them readable as
/// indirect arguments: dispatch at [`DISPATCH_AT`], draw at [`DRAW_AT`].
/// Our compute root signature and pipeline replace the list's. Call
/// [`done`] after the executions that read them.
pub(crate) fn write(
    list: &HostList,
    record: u64,
    max_primitives: u32,
    capacity: u32,
) -> Option<&'static ID3D12Resource> {
    let m = made()?;
    let l = &list.list;
    list.keep(&m.args);
    // SAFETY: an open list; our objects live as long as the process. The
    // buffer begins in the common state, which the write promotes.
    unsafe {
        l.SetComputeRootSignature(&m.root);
        l.SetPipelineState(&m.pso);
        l.SetComputeRoot32BitConstants(0, 2, [max_primitives, capacity].as_ptr().cast(), 0);
        l.SetComputeRootShaderResourceView(1, record);
        l.SetComputeRootUnorderedAccessView(2, m.args.GetGPUVirtualAddress());
        l.Dispatch(1, 1, 1);
        l.ResourceBarrier(&[transition(
            &m.args,
            D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
            D3D12_RESOURCE_STATE_INDIRECT_ARGUMENT,
        )]);
    }
    Some(&m.args)
}

/// The argument buffer back to the common state, for the next [`write`].
pub(crate) fn done(list: &HostList) {
    let Some(m) = made() else {
        return;
    };
    // SAFETY: an open list; the buffer lives as long as the process.
    unsafe {
        list.list.ResourceBarrier(&[transition(
            &m.args,
            D3D12_RESOURCE_STATE_INDIRECT_ARGUMENT,
            D3D12_RESOURCE_STATE_COMMON,
        )]);
    }
}
