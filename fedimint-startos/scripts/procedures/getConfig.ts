import { types as T, compat, matches } from "../deps.ts";

const DEFAULT_RUST_LOG = "info,jsonrpsee_core::client::async_client=off,hyper=off,h2=off,jsonrpsee_server=warn,jsonrpsee_server::transport=off,AlephBFT-=error,iroh=error";

const { shape, string } = matches;

const matchConfig = shape({
  "fedimintd-bitcoin-backend": shape({
    "backend-type": string,
  }),
});

const spec = {
  "fedimintd-bitcoin-backend": {
    type: "union",
    name: "Bitcoin Backend",
    description: "Choose how Fedimint connects to the Bitcoin network",
    "tag": {
      "id": "backend-type",
      "name": "Backend Type",
      "variant-names": {
        "bitcoind": "Bitcoin Core (Recommended)",
        "esplora": "Esplora"
      }
    },
    "default": "bitcoind",
    "variants": {
      "bitcoind": {
        "user": {
          type: "pointer",
          name: "RPC Username",
          description: "The username for Bitcoin Core's RPC interface",
          subtype: "package",
          "package-id": "bitcoind",
          target: "config",
          multi: false,
          selector: "$.rpc.username",
        },
        "password": {
          type: "pointer",
          name: "RPC Password",
          description: "The password for Bitcoin Core's RPC interface",
          subtype: "package",
          "package-id": "bitcoind",
          target: "config",
          multi: false,
          selector: "$.rpc.password",
        }
      },
      "esplora": {
        "url": {
          type: "string",
          name: "Esplora API URL",
          description: "The URL of the Esplora API to use (e.g., https://mempool.space/api)",
          nullable: false,
          default: "https://mempool.space/api",
          pattern: "^https?://.*",
          "pattern-description": "Must be a valid HTTP(S) URL"
        }
      }
    }
  },
  "advanced": {
    type: "object",
    name: "Advanced Settings",
    description: "Optional configuration for debugging and development",
    spec: {
      "rust-log-level": {
        type: "string",
        name: "Rust Log Directives",
        description: "Rust logging directives (e.g., 'info,fm=debug'). Only modify if debugging.",
        nullable: false,
        default: DEFAULT_RUST_LOG,
        pattern: ".*",
        "pattern-description": "Any valid Rust log directive string"
      }
    }
  }
} as const;

export const getConfig: T.ExpectedExports.getConfig = async (effects) => {
  const config = await effects.getConfig();

  // Transform old config to add missing advanced section
  if (matchConfig.test(config)) {
    const typedConfig = config as Record<string, unknown>;
    if (!typedConfig["advanced"]) {
      typedConfig["advanced"] = {
        "rust-log-level": DEFAULT_RUST_LOG
      };
    }
  }

  return {
    spec,
    config,
  };
};
