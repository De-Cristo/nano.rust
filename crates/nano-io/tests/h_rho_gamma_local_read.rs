use std::env;
use std::path::Path;

use nano_core::{BranchSchema, BranchSpec, BranchType};
use nano_io::events_chunked;

#[test]
fn reads_h_rho_gamma_nanoaod_v15_local_file_if_present() {
    let local_signal = match env::var("NANO_H_RHO_GAMMA_FILE") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("SKIP: NANO_H_RHO_GAMMA_FILE not set");
            return;
        }
    };
    let path = Path::new(&local_signal);
    if !path.exists() {
        eprintln!("SKIP: {} absent", path.display());
        return;
    }

    let schema = BranchSchema::new([
        BranchSpec::new("run", BranchType::U32),
        BranchSpec::new("luminosityBlock", BranchType::U32),
        BranchSpec::new("event", BranchType::U64),
        BranchSpec::new("nPhoton", BranchType::I32),
        BranchSpec::new("Photon_pt", BranchType::VecF32),
        BranchSpec::new("Photon_eta", BranchType::VecF32),
        BranchSpec::new("Photon_phi", BranchType::VecF32),
        BranchSpec::new("nPFCand", BranchType::I32),
        BranchSpec::new("PFCand_pt", BranchType::VecF32),
        BranchSpec::new("PFCand_eta", BranchType::VecF32),
        BranchSpec::new("PFCand_phi", BranchType::VecF32),
        BranchSpec::new("PFCand_pdgId", BranchType::VecI32),
        BranchSpec::new("PFCand_mass", BranchType::VecF32),
    ])
    .expect("schema");

    let mut rows = 0_usize;
    let mut plus_pions = 0_usize;
    let mut minus_pions = 0_usize;

    for event in events_chunked(path, &schema, 10)
        .expect("open local offline NanoAOD-like file")
        .take(10)
    {
        let event = event.expect("read event");
        let _run = event.scalar::<u32>("run").expect("run");
        let _lumi = event
            .scalar::<u32>("luminosityBlock")
            .expect("luminosityBlock");
        let _event = event.scalar::<u64>("event").expect("event");

        let n_photon = usize::try_from(event.scalar::<i32>("nPhoton").expect("nPhoton"))
            .expect("non-negative nPhoton");
        let photon_pt = event.vector_ref::<f32>("Photon_pt").expect("Photon_pt");
        let photon_eta = event.vector_ref::<f32>("Photon_eta").expect("Photon_eta");
        let photon_phi = event.vector_ref::<f32>("Photon_phi").expect("Photon_phi");
        assert_eq!(photon_pt.len(), n_photon);
        assert_eq!(photon_eta.len(), n_photon);
        assert_eq!(photon_phi.len(), n_photon);

        let n_pfcand = usize::try_from(event.scalar::<i32>("nPFCand").expect("nPFCand"))
            .expect("non-negative nPFCand");
        let pfcand_pt = event.vector_ref::<f32>("PFCand_pt").expect("PFCand_pt");
        let pfcand_eta = event.vector_ref::<f32>("PFCand_eta").expect("PFCand_eta");
        let pfcand_phi = event.vector_ref::<f32>("PFCand_phi").expect("PFCand_phi");
        let pfcand_pdg_id = event
            .vector_ref::<i32>("PFCand_pdgId")
            .expect("PFCand_pdgId");
        let pfcand_mass = event.vector_ref::<f32>("PFCand_mass").expect("PFCand_mass");
        assert_eq!(pfcand_pt.len(), n_pfcand);
        assert_eq!(pfcand_eta.len(), n_pfcand);
        assert_eq!(pfcand_phi.len(), n_pfcand);
        assert_eq!(pfcand_pdg_id.len(), n_pfcand);
        assert_eq!(pfcand_mass.len(), n_pfcand);

        plus_pions += pfcand_pdg_id.iter().filter(|&&id| id == 211).count();
        minus_pions += pfcand_pdg_id.iter().filter(|&&id| id == -211).count();
        rows += 1;
    }

    assert_eq!(rows, 10, "expected to read 10 events");
    eprintln!(
        "local offline NanoAODv15-like read: rows={rows}, +211={plus_pions}, -211={minus_pions}"
    );
}
