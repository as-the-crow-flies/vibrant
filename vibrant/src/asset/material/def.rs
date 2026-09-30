//! Optical properties of brain and head tissues, for volumetric rendering.
//!
//! UNITS AND CONVENTIONS
//! --------------------
//!   absorption  = mu_a   [mm^-1], per linear-sRGB channel
//!   scattering  = mu_s'  [mm^-1], REDUCED scattering, per linear-sRGB channel
//!   ior         = real refractive index (phase index), dimensionless
//!
//! These are NOT point samples of the underlying spectra. They are
//! appearance-matched fits: the full 380-780 nm spectra were built, the
//! resulting colour of the medium was computed under D65 with the CIE 1931
//! 2-degree observer, and the RGB triples below are the ones that reproduce
//! that colour in a three-channel renderer. See FITTING PROCEDURE below.
//!
//! mu_s' = (1 - g) * mu_s. If you drive a Henyey-Greenstein phase function
//! directly you must UN-REDUCE: mu_s = mu_s' / (1 - g). Brain tissue is strongly
//! forward scattering; measured g is 0.86-0.94 over 600-900 nm
//! [Ali & Bogdanovich 2023], so mu_s is roughly 7-15x the numbers below.
//! Per-tissue g is given in ANISOTROPY.
//!
//! FITTING PROCEDURE
//! -----------------
//! Point-sampling mu_a at 600/550/450 nm is wrong in a way that matters here,
//! because haemoglobin has enormous structure inside every sRGB primary and
//! because emergent light from a thick scattering medium is dominated by the
//! LEAST absorbed wavelengths in each band. A linear band average of mu_a
//! overcorrects in the opposite direction and comes out ~3x too dark in red.
//!
//! So instead, for each tissue:
//!
//!   1. build mu_a(lambda) and mu_s'(lambda) at 1 nm over 380-780 nm
//!   2. single-scattering albedo  a'(l) = mu_s' / (mu_s' + mu_a)
//!      diffusion attenuation     mu_eff(l) = sqrt(3 mu_a (mu_a + mu_s'))
//!   3. semi-infinite diffuse reflectance, van de Hulst similarity fit:
//!      R(a') = (1 - s)(1 - 0.139 s) / (1 + 1.17 s),   s = sqrt(1 - a')
//!      -> integrate R(lambda) against CIE 1931 x/y/z under D65, convert to
//!      linear sRGB. This is the true colour of a thick slab of the tissue.
//!   4. do the same for transmittance exp(-mu_eff * d) at d = one photometric
//!      diffusion length, giving the true transmitted colour.
//!   5. per channel, invert: R -> a'_c, T -> mu_eff_c, then
//!      mu_t'_c  = mu_eff_c / sqrt(3 (1 - a'_c))
//!      mu_a_c   = (1 - a'_c) mu_t'_c
//!      mu_s'_c  = a'_c mu_t'_c
//!
//! Two observables, two unknowns per channel, so the fit is exact: the RGB
//! material reproduces both the reflected and the transmitted colour of the
//! spectral medium. Same idea as the spectral-to-RGB parameter inversion used
//! for production subsurface scattering [Chiang, Kutz & Burley 2016].
//!
//! Because R(a') is used in both directions, any spectrally flat factor cancels
//! out of the fit -- in particular the internal Fresnel boost from the
//! n ~= 1.36-1.4 boundary, which is why it is safe to use the index-matched
//! form of the van de Hulst fit here.
//!
//! Measured against the spectral ground truth, the 600/550/450 point samples
//! were dE00 = 4.2 (grey matter), 6.1 (white matter), 6.5 (scalp), 7.9
//! (arterial blood). The values below are dE00 = 0 by construction.
//!
//! WHERE THE SPECTRA COME FROM
//! ---------------------------
//! Absorption in the visible is essentially all haemoglobin. Following the
//! generic-tissue model of [Jacques 2013]:
//!
//!   mu_a(lambda) = B [ S mu_a_oxy(lambda) + (1-S) mu_a_deoxy(lambda) ]
//!                + W mu_a_water(lambda)
//!
//! with the whole-blood coefficients for 150 g Hb / L from the Gratzer/Kollias
//! molar extinction tables compiled by S. Prahl [Prahl, omlc.org], water from
//! [Hale & Querry 1973], B the blood volume fraction and S the oxygen
//! saturation. B and S are from in vivo NIRS on head tissue
//! ([Matcher et al. 1997]: B = 3.35%, S = 64.1%; [O'Sullivan et al. 2012],
//! [Abookasis et al. 2009] on cortex: B = 3.0-3.8%, S = 59-61%). White matter
//! is scaled to B ~= 1.5% for its lower regional CBV.
//!
//! Reduced scattering uses [Jacques 2013] eq. 1 with the Table 1 fits:
//!
//!   mu_s'(lambda) = a (lambda / 500 nm)^-b        [a in cm^-1]
//!
//! Fit sources per tissue are named in the comments below.
//!
//! HEALTH WARNING
//! --------------
//! In vivo (Bevilacqua) and ex vivo (Yaroslavsky, Gebhart) measurements of brain
//! scattering disagree by close to an order of magnitude, mostly for white
//! matter. [Ali & Bogdanovich 2023] survey this and report mu_s = 2-20 mm^-1 for
//! grey and 20-50 mm^-1 for white matter over 600-900 nm, i.e. mu_s' ~= 0.2-2
//! and 2-5 mm^-1 at g = 0.9. This table uses the in vivo end of that range,
//! which is what head light-transport models use. If grey/white contrast looks
//! too weak, scaling WHITE_MATTER scattering up 2-3x is defensible against the
//! ex vivo literature.
//!
//! The fit is weakly conditioned wherever a channel is nearly black, because
//! the reflectance observable stops distinguishing absorption from scattering
//! there. Both blood entries are affected: the green and blue channels of blood
//! are opaque either way, so the mu_a / mu_s' split in those channels is close
//! to arbitrary. It is correct for how blood LOOKS and unreliable for transport
//! inside a large vessel lumen. If you are rendering the inside of vessels
//! rather than their surfaces, prefer mu_s' = [1.95, 2.07, 2.36] for both blood
//! entries -- the direct [Alexandrakis et al. 2005] fit -- and let mu_a absorb
//! the difference.

// ---------------------------------------------------------------- brain proper

use crate::asset::material::Material;

pub const AIR: Material = Material {
    absorption: [0.0, 0.0, 0.0],
    scattering: [0.0, 0.0, 0.0],
    ior: 1.0,
};

/// Cortical grey matter. B = 3.5%, S = 65%, W = 75%.
/// mu_s' fit: a = 10.9 cm^-1, b = 0.334, frontal-lobe cortex, in vivo
/// [Bevilacqua et al. 1999] via [Jacques 2013] Table 1 #10.
/// n = 1.3526 +- 0.0029 in vivo in rat somatosensory cortex at 1.1 um
/// [Binding et al. 2011]; rounded up slightly for visible dispersion.
/// slab colour (linear sRGB): [0.534, 0.199, 0.137]
pub const GRAY_MATTER: Material = Material {
    absorption: [0.0914, 0.6615, 1.1857],
    scattering: [1.1620, 1.0412, 1.0991],
    ior: 1.360,
};

/// White matter / corpus callosum. B = 1.5%, S = 65%, W = 72%.
/// mu_s' fit: a = 21.5 cm^-1, b = 1.629, cerebellar white matter, in vivo
/// [Bevilacqua et al. 1999] via [Jacques 2013] Table 1 #14.
/// n = 1.407 +- 0.015 for corpus callosum, OCT on acute rat slices
/// [Sun et al. 2012]; ~4% above surrounding grey, from myelin lipid content.
/// Myelin itself is modelled as alternating n = 1.47 / n = 1.35 lamellae.
/// slab colour (linear sRGB): [0.679, 0.430, 0.342]
pub const WHITE_MATTER: Material = Material {
    absorption: [0.0410, 0.2799, 0.5695],
    scattering: [1.4132, 1.9054, 2.3033],
    ior: 1.405,
};

/// Cerebellar cortex. B = 4.5% (densely vascularised granular layer), S = 65%.
/// mu_s' fit: a = 11.6 cm^-1, b = 0.601, temporal-lobe cortex
/// [Bevilacqua et al. 1999] via [Jacques 2013] Table 1 #11.
/// slab colour (linear sRGB): [0.497, 0.172, 0.122]
pub const CEREBELLAR_CORTEX: Material = Material {
    absorption: [0.1174, 0.8526, 1.4960],
    scattering: [1.1888, 1.0807, 1.1993],
    ior: 1.360,
};

/// Thalamus, basal ganglia, other deep grey. B = 4.0%, S = 65%.
/// Scattering borrowed from the cortex fit above; [Yaroslavsky et al. 2002]
/// measured thalamus and found it intermediate between grey and white matter.
/// slab colour (linear sRGB): [0.514, 0.187, 0.133]
pub const DEEP_GRAY_NUCLEI: Material = Material {
    absorption: [0.1044, 0.7569, 1.3427],
    scattering: [1.1710, 1.0849, 1.1979],
    ior: 1.370,
};

/// Pons / brainstem. B = 2.0%, S = 65%. Heavily myelinated, so it uses the
/// white matter scattering fit. [Yaroslavsky et al. 2002] measured pons
/// directly and found it close to white matter.
/// slab colour (linear sRGB): [0.646, 0.380, 0.301]
pub const PONS_BRAINSTEM: Material = Material {
    absorption: [0.0539, 0.3742, 0.7416],
    scattering: [1.4529, 1.9019, 2.3308],
    ior: 1.400,
};

/// Optic nerve / cranial nerve. B = 2.0%, S = 65%.
/// mu_s' fit: a = 25.9 cm^-1, b = 1.156, normal optic nerve
/// [Bevilacqua et al. 1999] via [Jacques 2013] Table 1 #13.
/// slab colour (linear sRGB): [0.683, 0.419, 0.319]
pub const OPTIC_NERVE: Material = Material {
    absorption: [0.0547, 0.3726, 0.7594],
    scattering: [1.9341, 2.3806, 2.6757],
    ior: 1.400,
};

// ------------------------------------------------------------ fluid and vessels

/// Cerebrospinal fluid. Effectively clear: absorption is pure water
/// [Hale & Querry 1973], ~1e-4 mm^-1 across the visible.
/// [Custo et al. 2006] show any mu_s' from 0 up to ~0.3 mm^-1 is
/// indistinguishable in head models; 0.024 mm^-1 is the value used in the
/// standard adult head model of [Okada & Delpy 2003]. n taken as saline.
/// The per-channel spread below is fit noise -- a' is ~1 here and the inversion
/// is ill-conditioned. Flattening all three to 0.024 is fine.
/// slab colour (linear sRGB): [0.752, 0.890, 0.962]  (faintly blue-white)
pub const CSF: Material = Material {
    absorption: [0.0005, 0.0001, 0.0000],
    scattering: [0.0295, 0.0238, 0.0256],
    ior: 1.335,
};

/// Arterial blood, S = 98%, 150 g Hb / L. The bright scarlet.
/// mu_s' fit: a = 22.0 cm^-1, b = 0.660, whole blood
/// [Alexandrakis et al. 2005] via [Jacques 2013] Table 1 #42.
/// n = 1.36 over 680-930 nm [Faber et al. 2004; Tuchin 2015].
/// Out of sRGB gamut before clamping -- real arterial blood is more saturated
/// than the display can show. See the note on weak conditioning above.
/// slab colour (linear sRGB): [0.289, 0.010, 0.012]
pub const BLOOD_ARTERIAL: Material = Material {
    absorption: [1.2913, 18.6357, 30.7347],
    scattering: [3.7631, 0.9741, 1.8330],
    ior: 1.360,
};

/// Venous / cortical surface vein blood, S = 65%. The dark purple-red.
/// The slab colour is roughly half the luminance of arterial at nearly the same
/// hue -- venous blood reads as darker rather than bluer, which is the correct
/// and frequently mis-rendered behaviour.
/// slab colour (linear sRGB): [0.154, 0.016, 0.014]
pub const BLOOD_VENOUS: Material = Material {
    absorption: [2.9548, 19.0027, 29.4156],
    scattering: [3.2167, 1.5787, 2.1770],
    ior: 1.360,
};

// ---------------------------------------------------------------- coverings

/// Dura mater. B = 1.0%, S = 65%. Dense collagen, so it takes the fibrous-tissue
/// scattering fit (a = 30.1 cm^-1, b = 1.549, [Neuman & Jacques 1991] via
/// [Jacques 2013] Table 1 #47) rather than a brain fit. Dura was measured
/// directly by [Shapey et al. 2022] over 400-1800 nm if you want real numbers.
/// WEAKEST ENTRY in this table -- treat as a placeholder.
/// slab colour (linear sRGB): [0.756, 0.556, 0.452]
pub const DURA_MATER: Material = Material {
    absorption: [0.0285, 0.1852, 0.4016],
    scattering: [1.9003, 2.6975, 3.1037],
    ior: 1.400,
};

/// Skull / cranial bone. B = 2.0%, S = 65%, W = 35%.
/// mu_s' fit: a = 20.9 cm^-1, b = 0.537 [Firbank et al. 1993] via
/// [Jacques 2013] Table 1 #26. [Bevilacqua et al. 1999] measured skull much
/// lower (a = 9.5, b = 0.141) -- another large literature spread.
/// n = 1.55 for cortical bone [Ascenzi & Fabry 1959; Tuchin 2015].
/// slab colour (linear sRGB): [0.678, 0.391, 0.280]
pub const SKULL: Material = Material {
    absorption: [0.0543, 0.3725, 0.7533],
    scattering: [1.8590, 2.0116, 2.0678],
    ior: 1.555,
};

/// Scalp / dermis. B = 2.0%, S = 70%. No melanin term -- add it separately if
/// you need skin tone; see [Jacques 2013] on melanosome volume fraction.
/// mu_s' fit: a = 45.3 cm^-1, b = 1.292, dermis
/// [Salomatina et al. 2006] via [Jacques 2013] Table 1 #6.
/// n = 1.40 as a thickness-weighted mean over skin sublayers [Ding et al. 2006].
/// slab colour (linear sRGB): [0.746, 0.513, 0.405]
pub const SCALP: Material = Material {
    absorption: [0.0512, 0.3697, 0.7905],
    scattering: [3.1068, 4.1352, 4.6508],
    ior: 1.400,
};

// ---------------------------------------------------------------- pathology

/// Low-grade astrocytoma. B = 3.0%, S = 65%.
/// mu_s' fit: a = 20.0 cm^-1, b = 1.629, astrocytoma of the optic nerve
/// [Bevilacqua et al. 1999] via [Jacques 2013] Table 1 #12.
/// slab colour (linear sRGB): [0.588, 0.300, 0.237]
pub const ASTROCYTOMA: Material = Material {
    absorption: [0.0793, 0.5640, 1.0676],
    scattering: [1.4231, 1.7627, 2.2077],
    ior: 1.370,
};

/// Medulloblastoma. B = 4.0%, S = 65%. Very high scattering power (b = 3.254),
/// so it goes strongly blue-scattering -- visually distinct from normal tissue.
/// mu_s' fit: a = 41.8 cm^-1, b = 3.254 [Bevilacqua et al. 1999] via
/// [Jacques 2013] Table 1 #15.
/// slab colour (linear sRGB): [0.588, 0.353, 0.324]
pub const MEDULLOBLASTOMA: Material = Material {
    absorption: [0.1050, 0.7543, 1.4392],
    scattering: [1.8897, 3.2635, 5.2195],
    ior: 1.370,
};

// ---------------------------------------------------------------- anisotropy

/// Henyey-Greenstein g, same order as the materials above. Use
/// mu_s = mu_s' / (1 - g) if your phase function is not already reduced.
/// Brain values from the 0.86-0.94 range surveyed by [Ali & Bogdanovich 2023];
/// blood from [Friebel et al. 2006].
///
/// Caveat: the fit above assumes similarity theory, i.e. that only mu_s'
/// matters. That holds in the diffusive regime and degrades for thin structures
/// and near-source geometry, which is exactly where a high-g medium looks
/// different from its isotropic-equivalent. For cortical surface detail at
/// grazing angles you may need the unreduced mu_s and true g rather than
/// mu_s' and g = 0.
pub const ANISOTROPY: [f32; 14] = [
    0.88, // GRAY_MATTER
    0.90, // WHITE_MATTER
    0.88, // CEREBELLAR_CORTEX
    0.88, // DEEP_GRAY_NUCLEI
    0.90, // PONS_BRAINSTEM
    0.90, // OPTIC_NERVE
    0.90, // CSF (irrelevant, mu_s' ~= 0)
    0.98, // BLOOD_ARTERIAL
    0.98, // BLOOD_VENOUS
    0.90, // DURA_MATER
    0.92, // SKULL
    0.85, // SCALP
    0.88, // ASTROCYTOMA
    0.88, // MEDULLOBLASTOMA
];

// ---------------------------------------------------------------- REFERENCES
//
// Ali, J.H. & Bogdanovich, S. (2023). Optical property measurements in normal
//   human brain tissues: exploring discrepancies in the visible-NIR region.
//   Biomed. J. Sci. Tech. Res. 51(2). doi:10.26717/BJSTR.2023.51.008077
//
// Ascenzi, A. & Fabry, C. (1959). Technique for dissection and measurement of
//   refractive index of osteones. J. Biophys. Biochem. Cytol. 6, 139-142.
//
// Bevilacqua, F., Piguet, D., Marquet, P., Gross, J.D., Tromberg, B.J. &
//   Depeursinge, C. (1999). In vivo local determination of tissue optical
//   properties: applications to human brain. Appl. Opt. 38(22), 4939-4950.
//   doi:10.1364/AO.38.004939
//
// Binding, J., Ben Arous, J., Leger, J.-F., Gigan, S., Boccara, C. &
//   Bourdieu, L. (2011). Brain refractive index measured in vivo with high-NA
//   defocus-corrected full-field OCT and consequences for two-photon
//   microscopy. Opt. Express 19(6), 4833-4847. doi:10.1364/OE.19.004833
//
// Chiang, M.J.-Y., Kutz, P. & Burley, B. (2016). Practical and controllable
//   subsurface scattering for production path tracing. ACM SIGGRAPH Talks.
//   doi:10.1145/2897839.2927433   -- the spectral-to-RGB inversion pattern.
//
// Custo, A., Wells, W.M., Barnett, A.H., Hillman, E.M.C. & Boas, D.A. (2006).
//   Effective scattering coefficient of the cerebral spinal fluid in adult head
//   models for diffuse optical imaging. Appl. Opt. 45(19), 4747-4755.
//   doi:10.1364/AO.45.004747
//
// Ding, H., Lu, J.Q., Wooden, W.A., Kragel, P.J. & Hu, X.-H. (2006). Refractive
//   indices of human skin tissues at eight wavelengths and estimated dispersion
//   relations between 300 and 1600 nm. Phys. Med. Biol. 51(6), 1479-1489.
//
// Faber, D.J., Aalders, M.C.G., Mik, E.G., Hooper, B.A., van Gemert, M.J.C. &
//   van Leeuwen, T.G. (2004). Oxygen saturation-dependent absorption and
//   scattering of blood. Phys. Rev. Lett. 93, 028102.
//
// Friebel, M., Roggan, A., Mueller, G. & Meinke, M. (2006). Determination of
//   optical properties of human blood in the spectral range 250 to 1100 nm
//   using Monte Carlo simulations with hematocrit-dependent effective
//   scattering phase functions. J. Biomed. Opt. 11(3), 034021.
//
// Hale, G.M. & Querry, M.R. (1973). Optical constants of water in the 200 nm to
//   200 um wavelength region. Appl. Opt. 12(3), 555-563.
//
// van de Hulst, H.C. (1980). Multiple Light Scattering: Tables, Formulas and
//   Applications. Academic Press.  -- semi-infinite similarity reflectance.
//
// Jacques, S.L. (2013). Optical properties of biological tissues: a review.
//   Phys. Med. Biol. 58(11), R37-R61. doi:10.1088/0031-9155/58/11/R37
//   Tables 1-3 as CSV: https://omlc.org/news/dec14/Jacques_PMB2013/
//
// Matcher, S.J., Cope, M. & Delpy, D.T. (1997). In vivo measurements of the
//   wavelength dependence of tissue-scattering coefficients between 760 and
//   900 nm measured with time-resolved spectroscopy. Appl. Opt. 36, 386-396.
//
// Okada, E. & Delpy, D.T. (2003). Near-infrared light propagation in an adult
//   head model. I. Modeling of low-level scattering in the cerebrospinal fluid
//   layer. Appl. Opt. 42(16), 2906-2914.
//
// Prahl, S.A. Optical absorption of hemoglobin. Tabulated molar extinction
//   coefficients compiled from W.B. Gratzer (Med. Res. Council Labs) and
//   N. Kollias (Wellman Labs). https://omlc.org/spectra/hemoglobin/summary.html
//
// Shapey, J., Xie, Y., Nabavi, E., et al. (2022). Optical properties of human
//   brain and tumour tissue: an ex vivo study spanning the visible range to
//   beyond the second near-infrared window. J. Biophotonics 15(4), e202100072.
//   doi:10.1002/jbio.202100072  -- grey, white, pons, pituitary, dura, cranial
//   nerve and five tumour types, 400-1800 nm. Best single source if you want to
//   replace the borrowed fits above with directly measured spectra.
//
// Sun, J., Lee, S.J., Wu, L., Sarntinoranont, M. & Xie, H. (2012). Refractive
//   index measurement of acute rat brain tissue slices using optical coherence
//   tomography. Opt. Express 20(2), 1084-1095. doi:10.1364/OE.20.001084
//
// Tuchin, V.V. (2015). Tissue Optics: Light Scattering Methods and Instruments
//   for Medical Diagnosis, 3rd ed. SPIE Press.
//
// Yaroslavsky, A.N., Schulze, P.C., Yaroslavsky, I.V., Schober, R., Ulrich, F. &
//   Schwarzmaier, H.-J. (2002). Optical properties of selected native and
//   coagulated human brain tissues in vitro in the visible and near infrared
//   spectral range. Phys. Med. Biol. 47(12), 2059-2073.
//   doi:10.1088/0031-9155/47/12/305
