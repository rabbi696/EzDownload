import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { validateSafeArguments } from "../src/utils/security.ts";

describe("Frontend security validator tests", () => {
  it("allows safe ffmpeg arguments", () => {
    assert.deepEqual(validateSafeArguments("-c:v libx264 -preset fast"), { valid: true });
    assert.deepEqual(validateSafeArguments(null), { valid: true });
    assert.deepEqual(validateSafeArguments(""), { valid: true });
  });

  it("blocks dangerous execution tokens", () => {
    assert.equal(validateSafeArguments("--exec echo 1").valid, false);
    assert.equal(validateSafeArguments("--EXEC echo 1").valid, false);
    assert.equal(validateSafeArguments("--config-location /etc/passwd").valid, false);
    assert.equal(validateSafeArguments("--external-downloader aria2c").valid, false);
  });

  it("blocks shell escape syntax", () => {
    assert.equal(validateSafeArguments("-b:v 1000k; rm -rf /").valid, false);
    assert.equal(validateSafeArguments("test && whoami").valid, false);
    assert.equal(validateSafeArguments("test || calc").valid, false);
    assert.equal(validateSafeArguments("`id`").valid, false);
    assert.equal(validateSafeArguments("$(reboot)").valid, false);
  });
});
