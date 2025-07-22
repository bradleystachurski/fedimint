export const getConfig = async () => {
  return {
    "bitcoin-rpc-user": {
      "type": "string",
      "name": "Bitcoin RPC Username",
      "description": "Enter the RPC username from your Bitcoin Core config (found in Bitcoin Core > Config > RPC Settings > Username)",
      "nullable": false,
      "default": "bitcoin"
    },
    "bitcoin-rpc-password": {
      "type": "string",
      "name": "Bitcoin RPC Password",
      "description": "Enter the RPC password from your Bitcoin Core config (found in Bitcoin Core > Config > RPC Settings > RPC Password)",
      "nullable": false,
      "default": "",
      "masked": true
    }
  };
};

export const setConfig = async (config) => {
  return { result: {} };
};
