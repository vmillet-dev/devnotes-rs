/** Generated from the Rust enum, so a state added there breaks the shell until handled. */
export type { VaultState } from '@core/ipc/bindings';

/**
 * What `vault::validate` refuses below, said on the front too: before a second of derivation,
 * and before an export prompt sells a short phrase as protection.
 */
export const MINIMUM_PASSPHRASE_LENGTH = 8;
