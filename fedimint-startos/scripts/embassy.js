export const getConfig = async () => {
  return {
    "bitcoin": {
      "type": "pointer",
      "subtype": "package",
      "package-id": "bitcoind",
      "target": "config",
      "selector": "rpc",
      "name": "Bitcoin Core RPC",
      "description": "The Bitcoin Core RPC credentials"
    }
  };
};

export const setConfig = async (config) => {
  return { result: {} };
};
