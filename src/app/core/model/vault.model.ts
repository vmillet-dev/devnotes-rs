/** Generated from the Rust enum, so a state added there breaks the shell until handled. */
export type { VaultState } from '@core/ipc/bindings';

/**
 * What `vault::validate` refuses below, from Rust: the form says it before a second of
 * derivation, and before an export prompt sells a short phrase as protection.
 */
export { MINIMUM_PASSPHRASE_LENGTH } from '@core/ipc/bindings';
