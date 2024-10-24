use hex_literal::hex;
use std::collections::HashMap;
use std::future::Future;
use std::str::FromStr;
use web3::api::{Accounts, Eth, Namespace};
use web3::contract::tokens::{Detokenize, Tokenize};
use web3::contract::{Contract, Error, Options};
use web3::signing;
use web3::signing::{Key, SecretKey, SecretKeyRef};
use web3::transports::Http;
use web3::types::{Address, BlockId, TransactionReceipt};

pub enum WvmAvsResult<R> {
    Success(web3::contract::Result<R>),
    Err(String),
}

pub struct WvmAvsOperator {
    pub pk: String,
    pub pks: SecretKey,
    pub from: Address,
    pub contracts: HashMap<String, Contract<Http>>,
    pub transport: Http,
    pub accounts: Accounts<Http>,
}

impl WvmAvsOperator {
    pub fn new(http_transport_url: String, pk: Option<String>) -> Self {
        let transport = Http::new(http_transport_url.as_str()).unwrap();
        let pk = pk.unwrap_or_else(|| std::env::var("WVM_AVS_OPERATOR_PK").unwrap());
        let key = SecretKey::from_slice(hex::decode(&pk).unwrap().as_slice()).unwrap();
        let from: Address = SecretKeyRef::new(&key).address();

        Self {
            pk,
            pks: key,
            from,
            contracts: HashMap::new(),
            transport: transport.clone(),
            accounts: Accounts::new(transport),
        }
    }

    pub fn init_contract(&mut self, alias: String, contract_address: String, abi: &[u8]) {
        let contract = Contract::from_json(
            Eth::new(self.transport.clone()),
            Address::from_str(&contract_address).unwrap(),
            abi,
        )
        .unwrap();
        self.contracts.insert(alias, contract);
    }

    pub async fn query<P, R>(
        &self,
        contract_alias: String,
        fn_name: String,
        params: P,
        opts: Option<Options>,
    ) -> web3::contract::Result<R>
    where
        R: Detokenize,
        P: Tokenize,
    {
        if let Some(contract) = self.contracts.get(&contract_alias) {
            contract
                .query(
                    &fn_name,
                    params,
                    self.from,
                    opts.unwrap_or_else(|| Options::default()),
                    None,
                )
                .await
        } else {
            Err(Error::InterfaceUnsupported)
        }
    }

    pub async fn call(
        &self,
        contract_alias: String,
        fn_name: String,
        params: impl Tokenize,
        options: Option<Options>,
        confirmations: usize,
    ) -> web3::error::Result<TransactionReceipt> {
        if let Some(contract) = self.contracts.get(&contract_alias) {
            contract
                .signed_call_with_confirmations(
                    &fn_name,
                    params,
                    options.unwrap_or_else(|| Options::default()),
                    confirmations,
                    &self.pks,
                )
                .await
        } else {
            Err(web3::Error::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::WvmAvsOperator;

    #[tokio::test]
    pub async fn test_query_contract() {
        let mut operator = WvmAvsOperator::new(
            "http://localhost:8545".to_string(),
            Some("9234bd23a4180e3a37a565150b058e20987dceb6ac63d98a571ec8197222242c".to_string()),
        );
        let json = r#"[{"inputs":[{"internalType":"string","name":"txIdOrGatewayAndTxId","type":"string"}],"name":"read_from_arweave","outputs":[{"internalType":"bytes","name":"","type":"bytes"}],"stateMutability":"view","type":"function"}]"#;

        operator.init_contract(
            "ar-reader".to_string(),
            "0x1c08473a8e024f0b08f15ec7a501c2b9bf104cea".to_string(),
            json.as_bytes(),
        );
        let res = operator
            .query(
                "ar-reader".to_string(),
                "read_from_arweave".to_string(),
                String::from("bs318IdjLWQK7pF_bNIbJnpade8feD7yGAS8xIffJDI"),
                None,
            )
            .await;
        let a: Vec<u8> = res.unwrap();
        assert_eq!(a, b"Hello world".to_vec());
    }

    #[tokio::test]
    pub async fn test_write_to_contract() {
        let mut operator = WvmAvsOperator::new(
            "http://localhost:8545".to_string(),
            Some("9234bd23a4180e3a37a565150b058e20987dceb6ac63d98a571ec8197222242c".to_string()),
        );
        let json = r#"[{"inputs":[{"internalType":"string","name":"dataString","type":"string"}],"name":"upload_to_arweave","outputs":[{"internalType":"bytes","name":"","type":"bytes"}],"stateMutability":"view","type":"function"}]"#;

        operator.init_contract(
            "ar-writer".to_string(),
            "0x30ea9c09bc861e1ece4b1a90f06da9880ffbf07d".to_string(),
            json.as_bytes(),
        );
        let res = operator
            .call(
                "ar-writer".to_string(),
                "upload_to_arweave".to_string(),
                String::from(""),
                None,
                1,
            )
            .await;
        let a = res.unwrap();
        println!("{:?}", a);
    }
}