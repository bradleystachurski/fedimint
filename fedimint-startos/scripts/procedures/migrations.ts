import { compat, types as T } from "../deps.ts";

const DEFAULT_RUST_LOG = "info,jsonrpsee_core::client::async_client=off,hyper=off,h2=off,jsonrpsee_server=warn,jsonrpsee_server::transport=off,AlephBFT-=error,iroh=error";

export const migration: T.ExpectedExports.migration = compat.migrations
  .fromMapping(
    {
      "0.9.0": {
        up: compat.migrations.updateConfig(
          (config: any) => {
            config.advanced = {
              "rust-log-level": DEFAULT_RUST_LOG
            };
            return config;
          },
          false,
          { version: "0.9.1", type: "up" }
        ),
        down: () => {
          throw new Error("Cannot downgrade");
        },
      },
    },
    "0.9.1"
  );
