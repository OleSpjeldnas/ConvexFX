//! SP1 Prover for ConvexFX Local Laws
//!
//! This module integrates SP1 zkVM to generate proofs that ConvexFX clearing
//! results satisfy all local laws (predicates) before submission to Delta.
//!
//! ## Modes
//! - **Mock Mode** (default): Fast testing without SP1 SDK
//! - **Production Mode** (--features sp1): Real SP1 proving with zkVM

use crate::{DeltaIntegrationError, Result};
use convexfx_clearing::EpochSolution;
use convexfx_types::AssetId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "sp1")]
use sp1_sdk::{ProverClient, SP1Stdin};

// ELF binary of the SP1 program (only needed in production mode)
#[cfg(feature = "sp1")]
pub const CONVEXFX_SP1_ELF: &[u8] = include_bytes!(
    "../../convexfx-sp1-program/elf/riscv32im-succinct-zkvm-elf"
);

/// Input data structure for the SP1 program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearingProofInput {
    pub y_star: Vec<(u8, f64)>,
    pub prices: Vec<(u8, f64)>,
    pub fills: Vec<FillData>,
    pub initial_inventory: Vec<(u8, f64)>,
    pub final_inventory: Vec<(u8, f64)>,
    pub convergence_achieved: bool,
    pub final_step_norm_y: f64,
    pub final_step_norm_alpha: f64,
    pub inventory_risk: f64,
    pub price_tracking: f64,
    pub fill_incentive: f64,
    pub total_objective: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillData {
    pub fill_frac: f64,
    pub pay_asset: u8,
    pub recv_asset: u8,
    pub pay_units: f64,
    pub recv_units: f64,
}

/// SP1 Prover for ConvexFX local laws
pub struct ConvexFxSp1Prover {
    #[cfg(feature = "sp1")]
    client: ProverClient,
    #[cfg(not(feature = "sp1"))]
    _phantom: (),
}

impl ConvexFxSp1Prover {
    /// Create a new SP1 prover
    pub fn new() -> Self {
        #[cfg(feature = "sp1")]
        {
            tracing::info!("Creating SP1 prover with ProverClient (production mode)");
            Self {
                client: ProverClient::new(),
            }
        }
        
        #[cfg(not(feature = "sp1"))]
        {
            tracing::debug!("Creating SP1 prover in mock mode (for testing)");
            Self {
                _phantom: (),
            }
        }
    }
    
    /// Get the verification key for the ConvexFX local laws program
    /// 
    /// This vkey is submitted with the domain agreement to register
    /// the local laws with the Delta base layer.
    pub fn get_vkey(&self) -> Vec<u8> {
        #[cfg(feature = "sp1")]
        {
            tracing::info!("Extracting SP1 verification key from program");
            let (_, vkey) = self.client.setup(CONVEXFX_SP1_ELF);
            vkey.bytes32().to_vec()
        }
        
        #[cfg(not(feature = "sp1"))]
        {
            tracing::debug!("Returning mock verification key (32 bytes)");
            vec![0u8; 32]
        }
    }
    
    /// Generate a proof that the clearing solution satisfies all local laws
    pub fn prove_clearing(
        &self,
        solution: &EpochSolution,
        initial_inventory: &BTreeMap<AssetId, f64>,
    ) -> Result<Vec<u8>> {
        // Prepare input for SP1 program
        let input = self.prepare_input(solution, initial_inventory);
        
        // Validate locally before attempting to prove
        // This catches errors early without expensive proving
        self.validate_input(&input)?;
        
        #[cfg(feature = "sp1")]
        {
            tracing::info!("Generating SP1 proof for clearing solution (epoch {})", solution.epoch_id);
            
            // Write input to SP1 stdin
            let mut stdin = SP1Stdin::new();
            stdin.write(&input);
            
            // Generate proof
            let (proof, _) = self.client.prove(CONVEXFX_SP1_ELF, stdin)
                .map_err(|e| DeltaIntegrationError::DeltaSdk(format!("SP1 proof generation failed: {}", e)))?;
            
            let proof_bytes = proof.bytes();
            tracing::info!("SP1 proof generated successfully ({} bytes)", proof_bytes.len());
            Ok(proof_bytes)
        }
        
        #[cfg(not(feature = "sp1"))]
        {
            tracing::debug!("Returning mock proof (64 bytes) - use --features sp1 for production");
            Ok(vec![0u8; 64])
        }
    }
    
    /// Prepare input data for the SP1 program from clearing solution
    fn prepare_input(
        &self,
        solution: &EpochSolution,
        initial_inventory: &BTreeMap<AssetId, f64>,
    ) -> ClearingProofInput {
        ClearingProofInput {
            y_star: solution.y_star.iter()
                .map(|(asset, price)| (asset.index() as u8, *price))
                .collect(),
            prices: solution.prices.iter()
                .map(|(asset, price)| (asset.index() as u8, *price))
                .collect(),
            fills: solution.fills.iter()
                .map(|fill| FillData {
                    fill_frac: fill.fill_frac,
                    pay_asset: fill.pay_asset.index() as u8,
                    recv_asset: fill.recv_asset.index() as u8,
                    pay_units: fill.pay_units,
                    recv_units: fill.recv_units,
                })
                .collect(),
            initial_inventory: initial_inventory.iter()
                .map(|(asset, qty)| (asset.index() as u8, *qty))
                .collect(),
            final_inventory: solution.q_post.iter()
                .map(|(asset, qty)| (asset.index() as u8, *qty))
                .collect(),
            convergence_achieved: solution.diagnostics.convergence_achieved,
            final_step_norm_y: solution.diagnostics.final_step_norm_y,
            final_step_norm_alpha: solution.diagnostics.final_step_norm_alpha,
            inventory_risk: solution.objective_terms.inventory_risk,
            price_tracking: solution.objective_terms.price_tracking,
            fill_incentive: solution.objective_terms.fill_incentive,
            total_objective: solution.objective_terms.total,
        }
    }
    
    /// Validate input locally before proving
    /// This catches errors early without expensive ZKP generation
    fn validate_input(&self, input: &ClearingProofInput) -> Result<()> {
        const TOLERANCE_Y: f64 = 1e-5;
        const TOLERANCE_ALPHA: f64 = 1e-6;
        
        if !input.convergence_achieved {
            return Err(DeltaIntegrationError::ClearingFailed(
                "SCP algorithm did not converge".to_string()
            ));
        }
        
        if input.final_step_norm_y >= TOLERANCE_Y {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "Price step norm {} exceeds tolerance {}",
                input.final_step_norm_y, TOLERANCE_Y
            )));
        }
        
        if input.final_step_norm_alpha >= TOLERANCE_ALPHA {
            return Err(DeltaIntegrationError::ClearingFailed(format!(
                "Fill step norm {} exceeds tolerance {}",
                input.final_step_norm_alpha, TOLERANCE_ALPHA
            )));
        }
        
        Ok(())
    }
}

impl Default for ConvexFxSp1Prover {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convexfx_clearing::{Diagnostics, ObjectiveTerms};
    use convexfx_types::Fill;

    fn create_test_solution() -> EpochSolution {
        let mut y_star = BTreeMap::new();
        let mut prices = BTreeMap::new();
        let mut q_post = BTreeMap::new();

        for asset in AssetId::all() {
            let log_price: f64 = if *asset == AssetId::USD { 0.0 } else { 0.1 };
            y_star.insert(*asset, log_price);
            prices.insert(*asset, log_price.exp());
            q_post.insert(*asset, 10000.0);
        }

        EpochSolution {
            epoch_id: 1,
            y_star,
            prices,
            q_post,
            fills: Vec::new(),
            objective_terms: ObjectiveTerms {
                inventory_risk: 100.0,
                price_tracking: 50.0,
                fill_incentive: -20.0,
                total: 130.0,
            },
            diagnostics: Diagnostics {
                iterations: 3,
                convergence_achieved: true,
                final_step_norm_y: 1e-6,
                final_step_norm_alpha: 1e-7,
                qp_status: "Optimal".to_string(),
            },
        }
    }

    #[test]
    fn test_sp1_prover_creation() {
        let prover = ConvexFxSp1Prover::new();
        let vkey = prover.get_vkey();
        assert_eq!(vkey.len(), 32);
    }

    #[test]
    fn test_prepare_input() {
        let prover = ConvexFxSp1Prover::new();
        let solution = create_test_solution();
        let mut initial_inventory = BTreeMap::new();
        for asset in AssetId::all() {
            initial_inventory.insert(*asset, 10000.0);
        }

        let input = prover.prepare_input(&solution, &initial_inventory);

        assert_eq!(input.y_star.len(), AssetId::all().len());
        assert_eq!(input.prices.len(), AssetId::all().len());
        assert_eq!(input.convergence_achieved, true);
        assert!(input.final_step_norm_y < 1e-5);
    }

    #[test]
    fn test_prove_clearing_success() {
        let prover = ConvexFxSp1Prover::new();
        let solution = create_test_solution();
        let mut initial_inventory = BTreeMap::new();
        for asset in AssetId::all() {
            initial_inventory.insert(*asset, 10000.0);
        }

        let result = prover.prove_clearing(&solution, &initial_inventory);
        assert!(result.is_ok());
        
        let proof = result.unwrap();
        assert_eq!(proof.len(), 64);
    }

    #[test]
    fn test_validate_input_convergence_failure() {
        let prover = ConvexFxSp1Prover::new();
        let mut solution = create_test_solution();
        solution.diagnostics.convergence_achieved = false;

        let mut initial_inventory = BTreeMap::new();
        for asset in AssetId::all() {
            initial_inventory.insert(*asset, 10000.0);
        }

        let input = prover.prepare_input(&solution, &initial_inventory);
        let result = prover.validate_input(&input);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("did not converge"));
    }
}

