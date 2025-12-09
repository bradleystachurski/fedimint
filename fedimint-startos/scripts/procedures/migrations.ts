import { compat, types as T } from "../deps.ts";

export const migration: T.ExpectedExports.migration = compat.migrations
  .fromMapping(
    {
      "0.9.0": {
        up: compat.migrations.updateConfig(
          (config: any) => config,  // Don't modify config
          false,  // Return configured: false to skip validation
          { version: "0.9.1", type: "up" }
        ),
        down: () => {
          throw new Error("Cannot downgrade");
        },
      },
    },
    "0.9.1"
  );
