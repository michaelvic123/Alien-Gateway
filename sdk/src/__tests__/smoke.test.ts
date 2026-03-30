/**
 * Smoke test to verify the SDK module structure.
 * This test verifies that the Jest test infrastructure is working.
 */

import * as fs from "fs";
import * as path from "path";
import * as process from "process";

describe("SDK Smoke Test", () => {
  it("should have Jest configured and running", () => {
    // Verify Jest is working
    expect(true).toBe(true);
  });

  it("should have valid Jest configuration", () => {
    // Verify configuration is in place - use process.cwd() for absolute path
    const configPath = path.join(process.cwd(), "jest.config.ts");
    expect(fs.existsSync(configPath)).toBe(true);
  });

  it("should have smoke test file in __tests__ directory", () => {
    const testFilePath = path.join(process.cwd(), "src/__tests__/smoke.test.ts");
    expect(fs.existsSync(testFilePath)).toBe(true);
  });

  it("should have GitHub Actions workflow", () => {
    const workflowPath = path.join(process.cwd(), "../.github/workflows/sdk_tests.yml");
    expect(fs.existsSync(workflowPath)).toBe(true);
  });
});
