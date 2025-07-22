export const getConfig = async () => {
  return {
    "bitcoin": {
      "type": "union",
      "name": "Bitcoin Core",
      "description": "The Bitcoin Core node to connect to",
      "tag": {
        "id": "type",
        "name": "Type",
        "variant-names": {
          "internal": "Internal (Bitcoin Core)"
        }
      },
      "default": "internal",
      "variants": {
        "internal": {
          "user": {
            "type": "pointer",
            "name": "RPC Username",
            "description": "The username for Bitcoin Core RPC",
            "subtype": "package",
            "package-id": "bitcoind",
            "target": "config",
            "selector": "rpc.username"
          },
          "password": {
            "type": "pointer",
            "name": "RPC Password",
            "description": "The password for Bitcoin Core RPC",
            "subtype": "package",
            "package-id": "bitcoind",
            "target": "config",
            "selector": "rpc.password"
          }
        }
      }
    }
  };
};

export const setConfig = async (config) => {
  return { result: {} };
};
