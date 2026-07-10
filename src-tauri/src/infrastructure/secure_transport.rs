use std::fmt::{Debug, Formatter};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, ClientConnection, DigitallySignedStruct, ServerConfig, ServerConnection,
};
use rustls::{SignatureScheme, StreamOwned};
use sha2::{Digest, Sha256};

use crate::application::ports::CredentialStore;
use crate::db::connection::Db;
use crate::db::repository;
use crate::error::{AppError, AppResult};
use crate::infrastructure::credential_store::SystemCredentialStore;

const CERTIFICATE_SETTING: &str = "host_certificate_der";
const FINGERPRINT_SETTING: &str = "host_tls_certificate_fingerprint";
const KEY_SCOPE: &str = "host-transport";
const KEY_NAME: &str = "tls-private-key";
const IO_TIMEOUT: Duration = Duration::from_secs(10);

pub type ServerTlsStream = StreamOwned<ServerConnection, TcpStream>;
pub type ClientTlsStream = StreamOwned<ClientConnection, TcpStream>;

pub struct HostTlsIdentity {
    pub server_config: Arc<ServerConfig>,
    pub fingerprint: String,
}

pub struct ConnectedClient {
    pub stream: ClientTlsStream,
    pub fingerprint: String,
}

pub fn load_or_create_host_identity(db: &Db) -> AppResult<HostTlsIdentity> {
    let store = SystemCredentialStore;
    let stored = db.with_conn(|conn| {
        Ok((
            repository::get_setting(conn, CERTIFICATE_SETTING)?,
            repository::get_setting(conn, FINGERPRINT_SETTING)?,
        ))
    })?;
    let private_key = store.load_password(KEY_SCOPE, KEY_NAME)?;

    match (stored.0, stored.1, private_key) {
        (None, None, None) => create_host_identity(db, &store),
        (Some(cert), Some(fingerprint), Some(key)) => {
            build_host_identity(&cert, &key, &fingerprint)
        }
        _ => Err(AppError::Validation(
            "主机 TLS 身份不完整；为防止降级或身份替换，服务已拒绝启动".to_string(),
        )),
    }
}

fn create_host_identity(db: &Db, store: &dyn CredentialStore) -> AppResult<HostTlsIdentity> {
    let signing_key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519)
        .map_err(|error| tls_error("生成主机证书私钥", error))?;
    let params =
        rcgen::CertificateParams::new(vec!["aster.local".to_string(), "localhost".to_string()])
            .map_err(|error| tls_error("生成主机证书参数", error))?;
    let cert = params
        .self_signed(&signing_key)
        .map_err(|error| tls_error("生成主机证书", error))?;
    let certificate_der = cert.der().to_vec();
    let private_key_der = signing_key.serialize_der();
    let certificate = STANDARD.encode(&certificate_der);
    let private_key = STANDARD.encode(&private_key_der);
    let fingerprint = certificate_fingerprint(&certificate_der);
    let identity = build_host_identity(&certificate, &private_key, &fingerprint)?;

    store.save_password(KEY_SCOPE, KEY_NAME, &private_key)?;
    if let Err(error) = db.with_conn(|conn| {
        repository::set_setting(conn, CERTIFICATE_SETTING, &certificate)?;
        repository::set_setting(conn, FINGERPRINT_SETTING, &fingerprint)
    }) {
        let _ = store.delete_password(KEY_SCOPE, KEY_NAME);
        return Err(error);
    }
    Ok(identity)
}

fn build_host_identity(
    encoded_certificate: &str,
    encoded_private_key: &str,
    expected_fingerprint: &str,
) -> AppResult<HostTlsIdentity> {
    let certificate_der = decode(encoded_certificate, "主机证书")?;
    let private_key_der = decode(encoded_private_key, "主机证书私钥")?;
    let fingerprint = certificate_fingerprint(&certificate_der);
    if fingerprint != expected_fingerprint {
        return Err(AppError::Validation(
            "主机证书指纹与持久化记录不匹配，服务已拒绝启动".to_string(),
        ));
    }
    let certificate = CertificateDer::from(certificate_der);
    let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(private_key_der));
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate], private_key)
        .map_err(|error| tls_error("加载主机证书", error))?;
    Ok(HostTlsIdentity {
        server_config: Arc::new(config),
        fingerprint,
    })
}

pub fn accept(stream: TcpStream, config: Arc<ServerConfig>) -> AppResult<ServerTlsStream> {
    configure_socket(&stream)?;
    let connection =
        ServerConnection::new(config).map_err(|error| tls_error("初始化 TLS 服务连接", error))?;
    Ok(StreamOwned::new(connection, stream))
}

pub fn connect(
    address: &str,
    port: u16,
    expected_fingerprint: Option<&str>,
) -> AppResult<ConnectedClient> {
    let socket = TcpStream::connect((address, port))
        .map_err(|error| AppError::Validation(format!("连接主机失败：{error}")))?;
    configure_socket(&socket)?;
    let verifier = Arc::new(FingerprintVerifier::new(expected_fingerprint));
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    let server_name =
        ServerName::try_from("aster.local").map_err(|error| tls_error("解析 TLS 主机名", error))?;
    let connection = ClientConnection::new(Arc::new(config), server_name)
        .map_err(|error| tls_error("初始化 TLS 客户端连接", error))?;
    let mut stream = StreamOwned::new(connection, socket);
    while stream.conn.is_handshaking() {
        stream
            .conn
            .complete_io(&mut stream.sock)
            .map_err(|error| tls_error("TLS 握手", error))?;
    }
    let certificate = stream
        .conn
        .peer_certificates()
        .and_then(|certificates| certificates.first())
        .ok_or_else(|| AppError::Validation("主机未提供 TLS 证书".to_string()))?;
    Ok(ConnectedClient {
        fingerprint: certificate_fingerprint(certificate.as_ref()),
        stream,
    })
}

pub fn certificate_fingerprint(certificate_der: &[u8]) -> String {
    let digest = Sha256::digest(certificate_der);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn configure_socket(stream: &TcpStream) -> AppResult<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    Ok(())
}

fn decode(value: &str, label: &str) -> AppResult<Vec<u8>> {
    STANDARD
        .decode(value)
        .map_err(|error| tls_error(&format!("解析{label}"), error))
}

fn tls_error(action: &str, error: impl std::fmt::Display) -> AppError {
    AppError::Validation(format!("{action}失败：{error}"))
}

struct FingerprintVerifier {
    expected: Option<String>,
    provider: Arc<CryptoProvider>,
}

impl FingerprintVerifier {
    fn new(expected: Option<&str>) -> Self {
        Self {
            expected: expected.map(str::to_owned),
            provider: Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
        }
    }
}

impl Debug for FingerprintVerifier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FingerprintVerifier")
            .field("pinned", &self.expected.is_some())
            .finish()
    }
}

impl ServerCertVerifier for FingerprintVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if let Some(expected) = &self.expected {
            if certificate_fingerprint(end_entity.as_ref()) != *expected {
                return Err(rustls::Error::General(
                    "Aster 主机证书指纹不匹配".to_string(),
                ));
            }
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::certificate_fingerprint;

    #[test]
    fn fingerprint_is_stable_lowercase_sha256() {
        assert_eq!(
            certificate_fingerprint(b"aster"),
            "b9245a5badfe84a1742b4e1a177cd2df281e8a3345cc517df119361e3d1fb596"
        );
    }
}
