use franklin_crypto::bellman::pairing::bn256::{Fr, G1Affine};
use franklin_crypto::bellman::CurveAffine;
// use franklin_crypto::bellman::Field;

use crate::batch_proof::{BatchProof, Bn256BatchProof, MultiProof};
use crate::ipa_fr::config::IpaConfig;
use crate::ipa_fr::transcript::{Bn256Transcript, PoseidonBn256Transcript};

use super::proof::{CommitmentElements, Elements, ExtraProofData, MultiProofCommitments};
use super::trie::{AbstractKey, AbstractStem, IntoFieldElement, VerkleTree};

#[derive(Clone, Debug)]
pub struct VerkleProof<K, GA>
where
    K: AbstractKey,
    GA: CurveAffine,
{
    pub multi_proof: MultiProof<GA>, // multi-point argument
    pub commitments: Vec<GA>,        // commitments, sorted by their path in the tree
    pub extra_data_list: Vec<ExtraProofData<K>>,
    pub keys: Vec<K>,
    pub values: Vec<[u8; 32]>,
}

impl<K> VerkleProof<K, G1Affine>
where
    K: AbstractKey<Path = Vec<usize>>,
    K::Stem: AbstractStem<Path = Vec<usize>> + IntoFieldElement<Fr>,
{
    pub fn create(
        tree: &mut VerkleTree<K, G1Affine>,
        keys: &[K],
    ) -> anyhow::Result<(Self, Elements<Fr>)> {
        let transcript = PoseidonBn256Transcript::with_bytes(b"multi_proof");
        tree.compute_commitment()?;

        let MultiProofCommitments {
            commitment_elements,
            extra_data_list,
        } = tree.get_commitments_along_path(keys)?;

        let CommitmentElements {
            commitments,
            elements,
        } = commitment_elements;

        let mut values: Vec<[u8; 32]> = vec![];
        for k in keys {
            let val = tree
                .get(k)
                .ok_or_else(|| anyhow::anyhow!("key {:?} is not found in this tree", k))?;
            values.push(*val);
        }

        let multi_proof = Bn256BatchProof::create_proof(
            &commitments,
            &elements.fs,
            &elements.zs,
            transcript.into_params(),
            &tree.committer,
        )?;
        let proof = VerkleProof {
            multi_proof,
            commitments,
            extra_data_list,
            keys: keys.to_vec(),
            values,
        };

        Ok((proof, elements))
    }

    pub fn check(
        &self,
        zs: &[usize],
        ys: &[Fr],
        ipa_conf: &IpaConfig<G1Affine>,
    ) -> anyhow::Result<bool> {
        let transcript = PoseidonBn256Transcript::with_bytes(b"multi_proof");
        Bn256BatchProof::check_proof(
            self.multi_proof.clone(),
            &self.commitments.clone(),
            ys,
            zs,
            transcript.into_params(),
            ipa_conf,
        )
    }
}
