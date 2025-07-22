export const getConfig = async () => {
  return {
    "spec": {
      "rpcuser": {
        "type": "string",
        "name": "RPC Username",
        "description": "Bitcoin Core RPC username",
        "nullable": false,
        "default": "bitcoin"
      },
      "rpcpass": {
        "type": "string",
        "name": "RPC Password",
        "description": "Bitcoin Core RPC password",
        "nullable": false,
        "default": "",
        "masked": true
      }
    }
  };
};

export const setConfig = async (config) => {
  return { result: {} };
};
