pub const GET_PEER_CONNECTION_INFO_ENDPOINT: &str = "get_peer_connection_info";
pub const ADD_PEER_CONNECTION_INFO_ENDPOINT: &str = "add_peer_connection_info";
pub const AUDIT_ENDPOINT: &str = "audit";
pub const GUARDIAN_CONFIG_BACKUP_ENDPOINT: &str = "download_guardian_backup";
pub const AUTH_ENDPOINT: &str = "auth";

#[deprecated(note = "https://github.com/fedimint/fedimint/issues/6671")]
pub const AWAIT_OUTPUT_OUTCOME_ENDPOINT: &str = "await_output_outcome";
pub const BACKUP_ENDPOINT: &str = "backup";
pub const ADD_CONFIG_GEN_PEER_ENDPOINT: &str = "add_config_gen_peer";
pub const BACKUP_STATISTICS_ENDPOINT: &str = "backup_statistics";
pub const CHECK_BITCOIN_STATUS_ENDPOINT: &str = "check_bitcoin_status";
pub const CLIENT_CONFIG_ENDPOINT: &str = "client_config";
pub const CLIENT_CONFIG_JSON_ENDPOINT: &str = "client_config_json";
pub const SERVER_CONFIG_CONSENSUS_HASH_ENDPOINT: &str = "server_config_consensus_hash";
pub const SESSION_COUNT_ENDPOINT: &str = "session_count";
pub const AWAIT_SESSION_OUTCOME_ENDPOINT: &str = "await_session_outcome";
pub const AWAIT_SIGNED_SESSION_OUTCOME_ENDPOINT: &str = "await_signed_session_outcome";
pub const SESSION_STATUS_ENDPOINT: &str = "session_status";
pub const SESSION_STATUS_V2_ENDPOINT: &str = "signed_session_status";
pub const SHUTDOWN_ENDPOINT: &str = "shutdown";
pub const CONFIG_GEN_PEERS_ENDPOINT: &str = "config_gen_peers";
pub const CONSENSUS_CONFIG_GEN_PARAMS_ENDPOINT: &str = "consensus_config_gen_params";
pub const DEFAULT_CONFIG_GEN_PARAMS_ENDPOINT: &str = "default_config_gen_params";
pub const VERIFY_CONFIG_HASH_ENDPOINT: &str = "verify_config_hash";
pub const RECOVER_ENDPOINT: &str = "recover";
pub const SERVER_STATUS_ENDPOINT: &str = "server_status";
pub const START_DKG_ENDPOINT: &str = "start_dkg";
pub const RUN_DKG_ENDPOINT: &str = "run_dkg";
pub const RESET_SETUP_ENDPOINT: &str = "reset_setup";
pub const SET_CONFIG_GEN_CONNECTIONS_ENDPOINT: &str = "set_config_gen_connections";
pub const SET_CONFIG_GEN_PARAMS_ENDPOINT: &str = "set_config_gen_params";
pub const SET_PASSWORD_ENDPOINT: &str = "set_password";
pub const SET_LOCAL_PARAMS_ENDPOINT: &str = "set_local_params";
pub const START_CONSENSUS_ENDPOINT: &str = "start_consensus";
pub const STATUS_ENDPOINT: &str = "status";
pub const SUBMIT_TRANSACTION_ENDPOINT: &str = "submit_transaction";
pub const VERIFIED_CONFIGS_ENDPOINT: &str = "verified_configs";
pub const VERSION_ENDPOINT: &str = "version";
pub const AWAIT_TRANSACTION_ENDPOINT: &str = "await_transaction";
pub const INVITE_CODE_ENDPOINT: &str = "invite_code";
pub const FEDERATION_ID_ENDPOINT: &str = "federation_id";
pub const RESTART_FEDERATION_SETUP_ENDPOINT: &str = "restart_federation_setup";
pub const API_ANNOUNCEMENTS_ENDPOINT: &str = "api_announcements";
pub const SUBMIT_API_ANNOUNCEMENT_ENDPOINT: &str = "submit_api_announcement";
pub const SIGN_API_ANNOUNCEMENT_ENDPOINT: &str = "sign_api_announcement";
pub const FEDIMINTD_VERSION_ENDPOINT: &str = "fedimintd_version";
