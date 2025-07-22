export const getConfig = async () => {
  return {
    "bitcoin": {
      "type": "pointer",
      "subtype": "package",
      "package-id": "bitcoind",
      "target": "config",
      "interface": "rpc",
      "name": "Bitcoin Core",
      "description": "The Bitcoin Core node to connect to"
    }
  };
};

export const setConfig = async (config) => {
  return { result: {} };
};
