//! The vertex shader that draws what a mesh program run as a compute
//! program wrote (`lower::set_mesh_as_compute`, [`MeshCompute`]), for a host
//! without mesh shaders (a GeForce GTX 10 series card).
//!
//! The host draws `groups * max_primitives * 3` vertices of a triangle list
//! without vertex buffers. Vertex `i` is corner `i % 3` of primitive
//! `(i / 3) % max_primitives` of group `i / 3 / max_primitives` of the
//! batch: the shader reads that primitive's vertex index from the buffer,
//! then that vertex's outputs, and hands them on as its own. A culled
//! primitive has three indices 0: a triangle without area, which the host
//! draws nothing of. Per-primitive outputs are read from the primitive's
//! record and go out flat; the host takes a flat input from a triangle's
//! first vertex, and each of the three carries the same values.
//!
//! The outputs are the pixel shader's inputs, register for register, so the
//! two link; each takes the mesh output of the same name and index (or the
//! same system value). One the mesh program does not export reads zero.

use super::MeshCompute;
use super::bitcode::{Attr, Bin, Cast, Term, V};
use super::container::SigElement;
use super::shader::{SLOT_MESH_OUT, Shader};
use crate::ShaderError;
use crate::dxbc::{Container, FourCC};

/// Where an output's first column comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// Dword of the vertex record.
    Vertex(u32),
    /// Dword of the primitive record.
    Primitive(u32),
    /// Not exported by the mesh program.
    Zero,
}

/// The vertex shader (a DXIL container) for the mesh program `ms` drawn
/// with the translated pixel shader `ps` (its [`super::HostShader::dxil`]),
/// if the pipeline has one.
pub fn mesh_vs(ms: &MeshCompute, ps: Option<&[u8]>) -> Result<Vec<u8>, ShaderError> {
    let vertex = super::signature(&ms.outputs)?;
    let prims = super::signature(&ms.prims)?;
    let wanted = match ps {
        Some(ps) => {
            let c = Container::parse(ps)?;
            match c.part(FourCC::ISG1) {
                Some(part) => super::signature(part.data)?,
                None => Vec::new(),
            }
        }
        None => Vec::new(),
    };
    build(ms, &vertex, &prims, &wanted)
}

/// Whether two signature elements stand for the same value.
fn same(a: &SigElement, b: &SigElement) -> bool {
    if a.kind != 0 || b.kind != 0 {
        return a.kind == b.kind && a.index == b.index;
    }
    a.name.eq_ignore_ascii_case(&b.name) && a.index == b.index
}

fn build(
    ms: &MeshCompute,
    vertex: &[SigElement],
    prims: &[SigElement],
    ps_inputs: &[SigElement],
) -> Result<Vec<u8>, ShaderError> {
    let unsupported = |what| ShaderError::Unsupported { offset: 0, what };
    if ms.max_primitives == 0 || ms.vertex_dwords == 0 {
        return Err(unsupported("a mesh program run as a compute program without records"));
    }
    let Some(position) = vertex.iter().find(|e| e.kind == 3 && e.row.is_some()) else {
        return Err(unsupported("a mesh program without a position"));
    };
    let mut outputs: Vec<SigElement> = Vec::new();
    let mut sources = Vec::new();
    // The pixel shader's interpolated inputs, in its registers. The
    // rasterizer's own values (front face, primitive id, sample index,
    // coverage, barycentrics) are none of ours.
    for e in ps_inputs.iter().filter(|e| matches!(e.kind, 0 | 3 | 6 | 7) && e.row.is_some()) {
        let source = if let Some(v) = vertex.iter().find(|v| same(v, e) && v.row.is_some()) {
            Source::Vertex(v.row.unwrap_or(0) * 4 + u32::from(e.col.max(v.col)))
        } else if let Some(p) = prims.iter().find(|p| same(p, e) && p.row.is_some()) {
            Source::Primitive(4 + p.row.unwrap_or(0) * 4 + u32::from(e.col.max(p.col)))
        } else {
            Source::Zero
        };
        outputs.push(SigElement {
            usage: (1 << e.cols) - 1,
            ..e.clone()
        });
        sources.push(source);
    }
    // A position even for a pixel shader that reads none (or for none).
    if !outputs.iter().any(|e| e.kind == 3) {
        let free = outputs.iter().filter_map(|e| e.row).map(|r| r + 1).max().unwrap_or(0);
        outputs.push(SigElement {
            interp: 4,
            row: Some(free),
            col: 0,
            cols: 4,
            usage: 15,
            ..position.clone()
        });
        sources.push(Source::Vertex(position.row.unwrap_or(0) * 4 + u32::from(position.col)));
    }

    let mut s = Shader::new(false);
    s.inputs = vec![SigElement {
        name: "SV_VertexID".into(),
        index: 0,
        kind: 1,
        comp: 5,
        interp: 0,
        row: Some(0),
        col: 0,
        cols: 1,
        usage: 1,
        half: false,
    }];
    s.outputs = outputs;
    let b = s.entry;
    let k = |s: &mut Shader, v: u32| s.m.i32(v);

    let args = [k(&mut s, 0), k(&mut s, 0), s.m.i8(0), s.m.undef(s.t.i32)];
    let id = s.op(b, "loadInput.i32", Attr::ReadNone, s.t.i32, 4, &args);
    let three = k(&mut s, 3);
    let triangle = s.m.bin(b, Bin::UDiv, id, three);
    let whole = s.m.bin(b, Bin::Mul, triangle, three);
    let corner = s.m.bin(b, Bin::Sub, id, whole);
    let most = k(&mut s, ms.max_primitives);
    let group = s.m.bin(b, Bin::UDiv, triangle, most);
    let primitive = s.m.bin(b, Bin::URem, triangle, most);
    let n = k(&mut s, ms.group_dwords());
    let group = s.m.bin(b, Bin::Mul, group, n);
    // The primitive's record and, by its index for this corner, the vertex's.
    let n = k(&mut s, ms.primitive_dwords);
    let at = s.m.bin(b, Bin::Mul, primitive, n);
    let n = k(&mut s, ms.max_vertices * ms.vertex_dwords);
    let at = s.m.bin(b, Bin::Add, at, n);
    let primitive_at = s.m.bin(b, Bin::Add, group, at);
    let h = s.root_srv_handle(SLOT_MESH_OUT);
    let index_at = s.m.bin(b, Bin::Add, primitive_at, corner);
    let index = load(&mut s, h, index_at);
    let index = s.m.cast(b, Cast::Bitcast, index, s.t.i32);
    let n = k(&mut s, ms.vertex_dwords);
    let at = s.m.bin(b, Bin::Mul, index, n);
    let vertex_at = s.m.bin(b, Bin::Add, group, at);

    for (id, source) in sources.iter().enumerate() {
        let float = s.outputs[id].comp == 9;
        for col in 0..s.outputs[id].cols {
            let value = match *source {
                Source::Zero => None,
                Source::Vertex(d) | Source::Primitive(d) => {
                    let record = if matches!(source, Source::Vertex(_)) {
                        vertex_at
                    } else {
                        primitive_at
                    };
                    let d = k(&mut s, d + u32::from(col));
                    let at = s.m.bin(b, Bin::Add, record, d);
                    Some(load(&mut s, h, at))
                }
            };
            let (name, value) = match (float, value) {
                (true, Some(v)) => ("storeOutput.f32", v),
                (true, None) => ("storeOutput.f32", s.m.f32(0)),
                (false, Some(v)) => ("storeOutput.i32", s.m.cast(b, Cast::Bitcast, v, s.t.i32)),
                (false, None) => ("storeOutput.i32", s.m.i32(0)),
            };
            let args = [k(&mut s, id as u32), k(&mut s, 0), s.m.i8(col), value];
            s.op(b, name, Attr::NoUnwind, s.t.void, 5, &args);
        }
    }
    s.m.terminate(b, Term::Ret);
    Ok(s.finish())
}

/// The dword at dword `at` of the buffer `h`, as a float.
fn load(s: &mut Shader, h: V, at: V) -> V {
    let b = s.entry;
    let two = s.m.i32(2);
    let bytes = s.m.bin(b, Bin::Shl, at, two);
    let (iu, mask, align) = (s.m.undef(s.t.i32), s.m.i8(1), s.m.i32(4));
    let ret = s.op(
        b,
        "rawBufferLoad.f32",
        Attr::ReadOnly,
        s.t.res_ret,
        139,
        &[h, bytes, iu, mask, align],
    );
    s.m.extract(b, ret, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(name: &str, index: u32, kind: u8, comp: u8, row: u32, cols: u8) -> SigElement {
        SigElement {
            name: name.into(),
            index,
            kind,
            comp,
            interp: 0,
            row: Some(row),
            col: 0,
            cols,
            usage: 0,
            half: false,
        }
    }

    fn mesh() -> (MeshCompute, Vec<SigElement>, Vec<SigElement>) {
        let vertex = vec![
            element("SV_Position", 0, 3, 9, 0, 4),
            element("TEXCOORD", 0, 0, 9, 1, 4),
        ];
        let prims = vec![element("TEXCOORD", 1, 0, 5, 0, 1)];
        let ms = MeshCompute {
            threads: 128,
            max_vertices: 64,
            max_primitives: 126,
            vertex_dwords: 8,
            primitive_dwords: 8,
            outputs: super::super::container::signature(&vertex, false),
            prims: super::super::container::signature(&prims, false),
        };
        (ms, vertex, prims)
    }

    /// The signatures a mesh program's record keeps come back as they went.
    #[test]
    fn the_records_signatures_read_back() {
        let (ms, vertex, prims) = mesh();
        let back = super::super::signature(&ms.outputs).unwrap();
        assert_eq!(back.len(), vertex.len());
        assert_eq!((back[1].row, back[1].cols, back[1].kind), (Some(1), 4, 0));
        let back = super::super::signature(&ms.prims).unwrap();
        assert_eq!((back[0].name.as_str(), back[0].index, back[0].comp), ("TEXCOORD", 1, prims[0].comp));
    }

    /// A vertex shader with the pixel shader's inputs as outputs, register
    /// for register (a vertex output, a primitive output, a position
    /// added), and the vertex id as its input; the validator, where there
    /// is one, accepts it.
    #[test]
    fn the_stage_is_a_vertex_shader_with_the_pixel_shaders_inputs() {
        let (ms, vertex, prims) = mesh();
        let ps = [
            SigElement { interp: 2, ..element("TEXCOORD", 0, 0, 9, 0, 4) },
            SigElement { interp: 1, ..element("TEXCOORD", 1, 0, 5, 1, 1) },
            SigElement { interp: 2, ..element("COLOR", 0, 0, 9, 2, 4) },
        ];
        let vs = build(&ms, &vertex, &prims, &ps).unwrap();
        let c = Container::parse(&vs).unwrap();
        // Program version: vertex (1), shader model 6.6.
        assert_eq!(c.part(FourCC::DXIL).unwrap().data[..4], [0x66, 0, 1, 0]);
        let outs = super::super::signature(c.part(FourCC::OSG1).unwrap().data).unwrap();
        assert_eq!(outs.len(), 4);
        for (o, p) in outs.iter().zip(&ps) {
            assert_eq!((&o.name, o.index, o.row, o.cols), (&p.name, p.index, p.row, p.cols));
        }
        assert_eq!((outs[3].kind, outs[3].row), (3, Some(3)));
        let ins = super::super::signature(c.part(FourCC::ISG1).unwrap().data).unwrap();
        assert_eq!((ins.len(), ins[0].kind), (1, 1));
        #[cfg(windows)]
        if let Ok(mut v) = super::super::validator::Validator::load(
            super::super::validator::Validator::default_path().as_deref(),
        ) {
            v.validate(&mut vs.clone()).unwrap();
            let alone = build(&ms, &vertex, &prims, &[]).unwrap();
            v.validate(&mut alone.clone()).unwrap();
        }
    }

    /// Where each output reads from: the vertex record by row, the
    /// primitive record past its indices, zero where the mesh program has
    /// no such output.
    #[test]
    fn outputs_read_the_record_of_their_name() {
        let (ms, vertex, prims) = mesh();
        let missing = [element("SV_Position", 0, 3, 9, 0, 4)];
        assert!(build(&ms, &vertex[1..], &prims, &missing).is_err());
        let none = MeshCompute { max_primitives: 0, ..ms.clone() };
        assert!(build(&none, &vertex, &prims, &[]).is_err());
    }
}
