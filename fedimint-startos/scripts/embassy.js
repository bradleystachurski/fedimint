export const getConfig = async () => {
  return {
    "bitcoind": {
      "type": "union",
      "name": "Bitcoin Core",
      "description": "The Bitcoin Core node to connect to",
      "tag": {
        "id": "type",
        "name": "Bitcoin Node Type",
        "variant-names": {
          "internal": "Bitcoin Core"
        }
      },
      "default": "internal",
      "variants": {
        "internal": {
          "user": {
            "type": "pointer",
            "name": "RPC Username",
            "description": "The username for Bitcoin Core's RPC interface",
            "subtype": "package",
            "package-id": "bitcoind",
            "target": "config",
            "multi": false,
            "selector": "$.rpc.username"
          },
          "password": {
            "type": "pointer",
            "name": "RPC Password",
            "description": "The password for Bitcoin Core's RPC interface",
            "subtype": "package",
            "package-id": "bitcoind",
            "target": "config",
            "multi": false,
            "selector": "$.rpc.password"
          }
        }
      }
    }
  };
};

export const setConfig = async (config) => {
  return { result: {} };
};
