use ff_utils::bn256_fr::Bn256Fr;
use franklin_crypto::babyjubjub::edwards::Point;
use franklin_crypto::babyjubjub::fs::Fs;
use franklin_crypto::bellman::pairing::bn256::{Bn256, Fr};
use franklin_crypto::bellman::PrimeField;
use generic_array::typenum;
use neptune::poseidon::PoseidonConstants;
use neptune::Poseidon;

use super::utils::{convert_fr_to_fs, convert_fs_to_fr, read_field_element_le};

pub trait Bn256Transcript: Sized + Clone {
    type Params;

    fn new(init_state: &Self::Params) -> Self;
    fn commit_field_element(&mut self, element: &Fs) -> anyhow::Result<()>;
    fn commit_point<Subgroup>(&mut self, point: &Point<Bn256, Subgroup>) -> anyhow::Result<()>;
    fn into_params(self) -> Self::Params;
    fn get_challenge(&self) -> Fs;
}

#[derive(Clone)]
pub struct PoseidonBn256Transcript {
    pub state: Bn256Fr,
}

#[test]
fn test_fs_poseidon_hash() {
    let constants = PoseidonConstants::new();
    let mut preimage = vec![<Bn256Fr as ff::Field>::ZERO; 2];
    let input1: Fr = read_field_element_le(&[1]).unwrap();
    let input2: Fr = read_field_element_le(&[2]).unwrap();
    preimage[0] = convert_ff_ce_to_ff(input1).unwrap();
    preimage[1] = convert_ff_ce_to_ff(input2).unwrap();
    let mut h = Poseidon::<Bn256Fr, typenum::U2>::new_with_preimage(&preimage, &constants);
    let output = h.hash();
    println!("output: {:?}", output);
}

impl Bn256Transcript for PoseidonBn256Transcript {
    type Params = Fr;

    fn new(init_state: &Self::Params) -> Self {
        // let blake_2s_state = Blake2sTranscript::new();

        Self {
            state: convert_ff_ce_to_ff(*init_state).unwrap(),
        }
    }

    fn commit_field_element(&mut self, element: &Fs) -> anyhow::Result<()> {
        let element_fr = convert_fs_to_fr::<Bn256>(element)?;
        self.commit_fr(&element_fr)?;

        Ok(())
    }

    fn commit_point<Subgroup>(&mut self, point: &Point<Bn256, Subgroup>) -> anyhow::Result<()> {
        let (point_x, point_y) = point.into_xy();
        self.commit_fr(&point_x)?;
        self.commit_fr(&point_y)?;

        Ok(())
    }

    fn into_params(self) -> Self::Params {
        convert_ff_to_ff_ce(self.state).unwrap()
    }

    fn get_challenge(&self) -> Fs {
        convert_fr_to_fs::<Bn256>(&convert_ff_to_ff_ce(self.state).unwrap()).unwrap()
    }
}

impl PoseidonBn256Transcript {
    pub fn with_bytes(bytes: &[u8]) -> Self {
        let chunk_size = (Fr::NUM_BITS / 8) as usize;
        assert!(chunk_size != 0);
        assert!(bytes.len() <= chunk_size);
        let element = read_field_element_le::<Fr>(bytes).unwrap();

        Self {
            state: convert_ff_ce_to_ff(element).unwrap(),
        }
    }

    pub fn commit_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let chunk_size = (Fr::NUM_BITS / 8) as usize;
        assert!(chunk_size != 0);
        for b in bytes.chunks(chunk_size) {
            let element = read_field_element_le::<Fr>(b).unwrap();
            self.commit_fr(&element)?;
        }

        Ok(())
    }

    pub fn commit_fr(&mut self, element: &Fr) -> anyhow::Result<()> {
        let mut preimage = vec![<Bn256Fr as ff::Field>::ZERO; 2];
        let constants = PoseidonConstants::new();
        preimage[0] = self.state;
        preimage[1] = convert_ff_ce_to_ff(*element).unwrap();

        let mut h = Poseidon::<Bn256Fr, typenum::U2>::new_with_preimage(&preimage, &constants);
        self.state = h.hash();

        Ok(())
    }
}

// uncheck overflow
pub fn from_bytes_le<F: ff::PrimeField>(bytes: &[u8]) -> anyhow::Result<F> {
    let mut value = F::ZERO;
    let mut factor = F::ONE;
    for b in bytes {
        value += factor * F::from(*b as u64);
        factor *= F::from(256u64);
    }

    Ok(value)
}

pub fn to_bytes_le<F: ff::PrimeField>(scalar: &F) -> Vec<u8> {
    scalar.to_repr().as_ref().to_vec()
}

#[test]
fn test_read_write_ff_ce_fs() {
    let bytes = [
        206u8, 104, 6, 65, 140, 79, 39, 170, 187, 254, 154, 245, 57, 39, 73, 145, 82, 144, 26, 62,
        229, 65, 168, 197, 168, 198, 162, 203, 73, 241, 49, 5,
    ];
    let point = from_bytes_le::<Bn256Fr>(&bytes).unwrap();
    assert_eq!(
        format!("{:?}", point),
        "Bn256Fr(0x0531f149cba2c6a8c5a841e53e1a905291492739f59afebbaa274f8c410668ce)"
    );

    let recovered_bytes = to_bytes_le(&point);
    assert_eq!(recovered_bytes, bytes);
}

pub fn convert_ff_to_ff_ce(value: Bn256Fr) -> anyhow::Result<Fr> {
    read_field_element_le(&to_bytes_le(&value))
}

pub fn convert_ff_ce_to_ff(value: Fr) -> anyhow::Result<Bn256Fr> {
    from_bytes_le(&super::utils::write_field_element_le(&value))
}
