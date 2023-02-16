use crate::evm::FuzzHost;
use crate::middleware::MiddlewareOp::{UpdateCode, UpdateSlot};
use crate::middleware::{Middleware, MiddlewareOp};
use crate::onchain::endpoints::OnChainConfig;
use crate::types::convert_u256_to_h160;
use libafl::prelude::MutationResult;
use primitive_types::{H160, H256, U256};
use revm::Interpreter;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

const UNBOUND_THRESHOLD: usize = 5;

#[derive(Clone, Debug)]
pub struct OnChain {
    pub loaded_data: HashSet<(H160, U256)>,
    pub loaded_code: HashSet<H160>,
    pub calls: HashMap<(H160, usize), usize>,
    pub endpoint: OnChainConfig,
}

impl OnChain {
    pub fn new(endpoint: OnChainConfig) -> Self {
        Self {
            loaded_data: Default::default(),
            loaded_code: Default::default(),
            calls: Default::default(),
            endpoint,
        }
    }
}

impl Middleware for OnChain {
    unsafe fn on_step(&mut self, interp: &mut Interpreter) -> Vec<MiddlewareOp> {
        match *interp.instruction_pointer {
            0x54 => {
                let slot_idx = interp.stack.peek(0).unwrap();
                let address = interp.contract.address;
                if self.loaded_data.contains(&(address, slot_idx)) {
                    vec![]
                } else {
                    self.loaded_data.insert((address, slot_idx));
                    vec![UpdateSlot(
                        address,
                        slot_idx,
                        self.endpoint.get_contract_slot(address, slot_idx),
                    )]
                }
            }

            0xf1 | 0xf2 | 0xf4 | 0xfa => {
                let pc = interp.program_counter();
                let calls_data = self.calls.get_mut(&(interp.contract.address, pc));
                match calls_data {
                    None => {
                        self.calls.insert((interp.contract.address, pc), 1);
                    }
                    Some(v) => {
                        if *v > UNBOUND_THRESHOLD {
                            return vec![];
                        }
                        *v += 1;
                    }
                }
                let address = interp.stack.peek(1).unwrap();
                let address_h160 = convert_u256_to_h160(address);
                if self.loaded_code.contains(&address_h160) {
                    vec![]
                } else {
                    self.loaded_code.insert(address_h160);
                    vec![UpdateCode(
                        address_h160,
                        self.endpoint.get_contract_code(address_h160),
                    )]
                }
            }
            _ => {
                vec![]
            }
        }
    }
}