use std::collections::HashMap;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use opaque_ke::argon2::Argon2;
use opaque_ke::ciphersuite::CipherSuite;
use opaque_ke::rand::rngs::OsRng;
use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialFinalization, CredentialRequest,
    CredentialResponse, RegistrationRequest, RegistrationResponse, RegistrationUpload, ServerLogin,
    ServerLoginParameters, ServerRegistration, ServerSetup,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const PROTOCOL_ID: &str = "aster-pair-v2";
const CREDENTIAL_ID: &[u8] = b"aster-one-time-pairing";
const EXCHANGE_TTL: Duration = Duration::from_secs(120);

pub struct AsterCipherSuite;

impl CipherSuite for AsterCipherSuite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::TripleDh<opaque_ke::Ristretto255, sha2::Sha512>;
    type Ksf = Argon2<'static>;
}

#[derive(Default)]
pub struct PairingRuntime {
    registration: Option<PairingRegistration>,
    exchanges: HashMap<String, PendingExchange>,
}

struct PairingRegistration {
    id: String,
    code: String,
    server_setup: Vec<u8>,
    password_file: Vec<u8>,
}

struct PendingExchange {
    registration_id: String,
    client_name: String,
    client_device_id: String,
    app_version: String,
    client_ip: String,
    context: Vec<u8>,
    server_state: Vec<u8>,
    created_at: Instant,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairStartRequest {
    pub client_name: String,
    pub client_device_id: String,
    pub app_version: String,
    pub client_nonce: String,
    pub ke1: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairStartResponse {
    pub exchange_id: String,
    pub server_nonce: String,
    pub ke2: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairFinishRequest {
    pub exchange_id: String,
    pub ke3: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairFinishResponse {
    pub token: String,
    pub message: String,
}

pub struct VerifiedPairing {
    pub client_name: String,
    pub client_device_id: String,
    pub app_version: String,
    pub client_ip: String,
}

impl PairingRuntime {
    pub fn initialize() -> AppResult<Self> {
        let mut runtime = Self::default();
        runtime.rotate()?;
        Ok(runtime)
    }

    pub fn code(&self) -> Option<&str> {
        self.registration
            .as_ref()
            .map(|registration| registration.code.as_str())
    }

    pub fn begin(
        &mut self,
        request: PairStartRequest,
        client_ip: String,
        certificate_fingerprint: &str,
    ) -> AppResult<PairStartResponse> {
        self.remove_expired();
        validate_client_metadata(&request.client_name, &request.client_device_id)?;
        validate_nonce(&request.client_nonce)?;
        let registration = self.registration.as_ref().ok_or_else(pairing_failed)?;
        let server_nonce = random_nonce();
        let context = pairing_context(
            certificate_fingerprint,
            &request.client_device_id,
            &request.client_nonce,
            &server_nonce,
        );
        let setup = ServerSetup::<AsterCipherSuite>::deserialize(&registration.server_setup)
            .map_err(|_| pairing_failed())?;
        let password_file =
            ServerRegistration::<AsterCipherSuite>::deserialize(&registration.password_file)
                .map_err(|_| pairing_failed())?;
        let credential_request =
            CredentialRequest::<AsterCipherSuite>::deserialize(&decode_message(&request.ke1)?)
                .map_err(|_| pairing_failed())?;
        let parameters = ServerLoginParameters {
            context: Some(&context),
            ..ServerLoginParameters::default()
        };
        let started = ServerLogin::start(
            &mut OsRng,
            &setup,
            Some(password_file),
            credential_request,
            CREDENTIAL_ID,
            parameters,
        )
        .map_err(|_| pairing_failed())?;
        let exchange_id = Uuid::new_v4().to_string();
        self.exchanges.insert(
            exchange_id.clone(),
            PendingExchange {
                registration_id: registration.id.clone(),
                client_name: request.client_name,
                client_device_id: request.client_device_id,
                app_version: request.app_version,
                client_ip,
                context,
                server_state: started.state.serialize().to_vec(),
                created_at: Instant::now(),
            },
        );
        Ok(PairStartResponse {
            exchange_id,
            server_nonce,
            ke2: STANDARD.encode(started.message.serialize()),
        })
    }

    pub fn finish(&mut self, request: PairFinishRequest) -> AppResult<VerifiedPairing> {
        self.remove_expired();
        let pending = self
            .exchanges
            .remove(request.exchange_id.trim())
            .ok_or_else(pairing_failed)?;
        let current_id = self
            .registration
            .as_ref()
            .map(|registration| registration.id.as_str())
            .ok_or_else(pairing_failed)?;
        if pending.registration_id != current_id {
            return Err(pairing_failed());
        }
        let state = ServerLogin::<AsterCipherSuite>::deserialize(&pending.server_state)
            .map_err(|_| pairing_failed())?;
        let finalization =
            CredentialFinalization::<AsterCipherSuite>::deserialize(&decode_message(&request.ke3)?)
                .map_err(|_| pairing_failed())?;
        state
            .finish(
                finalization,
                ServerLoginParameters {
                    context: Some(&pending.context),
                    ..ServerLoginParameters::default()
                },
            )
            .map_err(|_| pairing_failed())?;
        let verified = VerifiedPairing {
            client_name: pending.client_name,
            client_device_id: pending.client_device_id,
            app_version: pending.app_version,
            client_ip: pending.client_ip,
        };
        self.rotate()?;
        Ok(verified)
    }

    fn rotate(&mut self) -> AppResult<()> {
        let code = generate_pair_code();
        let setup = ServerSetup::<AsterCipherSuite>::new(&mut OsRng);
        let client_started =
            ClientRegistration::<AsterCipherSuite>::start(&mut OsRng, code.as_bytes())
                .map_err(|_| pairing_failed())?;
        let server_started = ServerRegistration::<AsterCipherSuite>::start(
            &setup,
            RegistrationRequest::deserialize(&client_started.message.serialize())
                .map_err(|_| pairing_failed())?,
            CREDENTIAL_ID,
        )
        .map_err(|_| pairing_failed())?;
        let client_finished = client_started
            .state
            .finish(
                &mut OsRng,
                code.as_bytes(),
                RegistrationResponse::deserialize(&server_started.message.serialize())
                    .map_err(|_| pairing_failed())?,
                ClientRegistrationFinishParameters::default(),
            )
            .map_err(|_| pairing_failed())?;
        let password_file = ServerRegistration::<AsterCipherSuite>::finish(
            RegistrationUpload::<AsterCipherSuite>::deserialize(
                &client_finished.message.serialize(),
            )
            .map_err(|_| pairing_failed())?,
        );
        self.registration = Some(PairingRegistration {
            id: Uuid::new_v4().to_string(),
            code,
            server_setup: setup.serialize().to_vec(),
            password_file: password_file.serialize().to_vec(),
        });
        self.exchanges.clear();
        Ok(())
    }

    fn remove_expired(&mut self) {
        self.exchanges
            .retain(|_, exchange| exchange.created_at.elapsed() < EXCHANGE_TTL);
    }
}

pub fn client_start(password: &str) -> AppResult<(ClientLogin<AsterCipherSuite>, String)> {
    let started = ClientLogin::<AsterCipherSuite>::start(&mut OsRng, password.as_bytes())
        .map_err(|_| pairing_failed())?;
    Ok((started.state, STANDARD.encode(started.message.serialize())))
}

pub fn client_finish(
    state: ClientLogin<AsterCipherSuite>,
    password: &str,
    response: &PairStartResponse,
    certificate_fingerprint: &str,
    client_device_id: &str,
    client_nonce: &str,
) -> AppResult<String> {
    let context = pairing_context(
        certificate_fingerprint,
        client_device_id,
        client_nonce,
        &response.server_nonce,
    );
    let credential_response =
        CredentialResponse::<AsterCipherSuite>::deserialize(&decode_message(&response.ke2)?)
            .map_err(|_| pairing_failed())?;
    let finished = state
        .finish(
            &mut OsRng,
            password.as_bytes(),
            credential_response,
            ClientLoginFinishParameters::new(Some(&context), Default::default(), None),
        )
        .map_err(|_| pairing_failed())?;
    Ok(STANDARD.encode(finished.message.serialize()))
}

fn pairing_context(
    certificate_fingerprint: &str,
    client_device_id: &str,
    client_nonce: &str,
    server_nonce: &str,
) -> Vec<u8> {
    format!(
        "{PROTOCOL_ID}\0{certificate_fingerprint}\0{client_device_id}\0{client_nonce}\0{server_nonce}"
    )
    .into_bytes()
}

fn generate_pair_code() -> String {
    OsRng
        .gen_range(100_000_000_000_u64..1_000_000_000_000_u64)
        .to_string()
}

pub fn random_nonce() -> String {
    STANDARD.encode(Uuid::new_v4().as_bytes())
}

fn decode_message(value: &str) -> AppResult<Vec<u8>> {
    STANDARD.decode(value).map_err(|_| pairing_failed())
}

fn validate_nonce(nonce: &str) -> AppResult<()> {
    let decoded = decode_message(nonce)?;
    if decoded.len() != 16 {
        return Err(pairing_failed());
    }
    Ok(())
}

fn validate_client_metadata(client_name: &str, client_device_id: &str) -> AppResult<()> {
    if client_name.trim().is_empty() || client_name.chars().count() > 80 {
        return Err(pairing_failed());
    }
    if client_device_id.trim().is_empty() || client_device_id.chars().count() > 128 {
        return Err(pairing_failed());
    }
    Ok(())
}

fn pairing_failed() -> AppError {
    AppError::Validation("配对验证失败，请检查配对码后重试".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_exchange_consumes_pairing_code_and_rejects_replay() {
        let mut runtime = PairingRuntime::initialize().expect("pairing runtime");
        let code = runtime.code().expect("pairing code").to_string();
        let nonce = random_nonce();
        let (client, ke1) = client_start(&code).expect("client start");
        let started = runtime
            .begin(
                PairStartRequest {
                    client_name: "测试客户端".to_string(),
                    client_device_id: "device-1".to_string(),
                    app_version: "0.1.0".to_string(),
                    client_nonce: nonce.clone(),
                    ke1,
                },
                "127.0.0.1".to_string(),
                "fingerprint-a",
            )
            .expect("server start");
        let ke3 = client_finish(client, &code, &started, "fingerprint-a", "device-1", &nonce)
            .expect("client finish");
        let exchange_id = started.exchange_id;
        runtime
            .finish(PairFinishRequest {
                exchange_id: exchange_id.clone(),
                ke3: ke3.clone(),
            })
            .expect("server finish");
        assert!(runtime
            .finish(PairFinishRequest { exchange_id, ke3 })
            .is_err());
        assert_ne!(runtime.code(), Some(code.as_str()));
    }

    #[test]
    fn certificate_context_change_is_rejected() {
        let mut runtime = PairingRuntime::initialize().expect("pairing runtime");
        let code = runtime.code().expect("pairing code").to_string();
        let nonce = random_nonce();
        let (client, ke1) = client_start(&code).expect("client start");
        let started = runtime
            .begin(
                PairStartRequest {
                    client_name: "测试客户端".to_string(),
                    client_device_id: "device-1".to_string(),
                    app_version: "0.1.0".to_string(),
                    client_nonce: nonce.clone(),
                    ke1,
                },
                "127.0.0.1".to_string(),
                "fingerprint-a",
            )
            .expect("server start");
        assert!(
            client_finish(client, &code, &started, "fingerprint-b", "device-1", &nonce).is_err()
        );
    }
}
