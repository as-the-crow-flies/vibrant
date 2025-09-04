use crate::{asset::line::LineSet, file::LineFile, gpu::Gpu};

#[derive(Debug, Clone, Copy)]
pub enum BenchmarkLineSet {
    Aneurysm,
    Turbulence,
    Brain200k,
    Brain1M,
    BundlesSmall,
    BundlesBig,
}

impl BenchmarkLineSet {
    pub fn iter() -> Vec<BenchmarkLineSet> {
        vec![
            BenchmarkLineSet::Aneurysm,
            BenchmarkLineSet::Turbulence,
            BenchmarkLineSet::Brain200k,
            BenchmarkLineSet::Brain1M,
            BenchmarkLineSet::BundlesSmall,
            BenchmarkLineSet::BundlesBig,
        ]
    }
}

pub fn load_line(gpu: &Gpu, set: BenchmarkLineSet) -> LineSet {
    match set {
        BenchmarkLineSet::Aneurysm => load_line_aneurysm(gpu),
        BenchmarkLineSet::Turbulence => load_line_turbulence(gpu),
        BenchmarkLineSet::Brain200k => load_line_brain_200k(gpu, 1),
        BenchmarkLineSet::Brain1M => load_line_brain_1m(gpu, 1),
        BenchmarkLineSet::BundlesSmall => load_line_brain_bundles_small(gpu),
        BenchmarkLineSet::BundlesBig => load_line_brain_bundles_big(gpu),
    }
}

pub fn load_line_aneurysm(gpu: &Gpu) -> LineSet {
    LineSet::new(
        gpu,
        &LineFile::from_file("assets/3D_line_sets/ANEURYSM.obj"),
    )
}

pub fn load_line_turbulence(gpu: &Gpu) -> LineSet {
    LineSet::new(
        gpu,
        &LineFile::from_file("assets/3D_line_sets/TURBULENCE.obj"),
    )
}

pub fn load_line_brain_200k(gpu: &Gpu, stride: usize) -> LineSet {
    LineSet::new(
        gpu,
        &LineFile::from_tck_file("assets/HCP-100307/whole_brain200k.tck", stride),
    )
}

pub fn load_line_brain_1m(gpu: &Gpu, stride: usize) -> LineSet {
    LineSet::new(
        gpu,
        &LineFile::from_tck_file("assets/HCP-100307/whole_brain1M.tck", stride),
    )
}

pub fn load_line_brain_bundles_small(gpu: &Gpu) -> LineSet {
    LineSet::new(
        gpu,
        &LineFile::join(vec![
            LineFile::from_file("assets/HCP-100307/TOM_trackings/AF_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/AF_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ATR_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ATR_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CG_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CG_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CST_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CST_right.tck"),
        ]),
    )
}

pub fn load_line_brain_bundles_big(gpu: &Gpu) -> LineSet {
    LineSet::new(
        gpu,
        &LineFile::join(vec![
            LineFile::from_file("assets/HCP-100307/TOM_trackings/AF_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/AF_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ATR_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ATR_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CA.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_1.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_2.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_3.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_4.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_5.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_6.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_7.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CC.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CG_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CG_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CST_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/CST_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/FPT_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/FPT_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/FX_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/FX_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ICP_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ICP_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/IFO_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/IFO_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ILF_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ILF_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/MCP.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/MLF_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/MLF_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/OR_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/OR_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/POPT_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/POPT_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/SCP_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/SCP_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_I_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_I_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_II_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_II_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_III_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_III_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_FO_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_FO_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_OCC_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_OCC_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PAR_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PAR_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_POSTC_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_POSTC_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREC_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREC_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREF_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREF_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREM_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREM_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/STR_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/STR_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_OCC_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_OCC_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PAR_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PAR_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_POSTC_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_POSTC_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREC_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREC_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREF_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREF_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREM_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREM_right.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/UF_left.tck"),
            LineFile::from_file("assets/HCP-100307/TOM_trackings/UF_right.tck"),
        ]),
    )
}
