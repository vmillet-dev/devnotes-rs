/**
 * A copy of the library, taken at launch.
 *
 * ⚠️ A conversion exists only where the wire shape differs from the model shape, and here
 * it does: JSON has no date type, so `takenAt` crosses as an ISO string.
 */
export interface Backup {
  /** The folder's name, which is its stamp — and what a restore is asked for by. */
  readonly id: string;
  readonly takenAt: Date;
  readonly bytes: number;
  /** Whether the key file travelled with it. Without one the copy opens for nobody. */
  readonly openable: boolean;
}
