/** A copy of the library, taken at launch; `takenAt` crosses as an ISO string. */
export interface Backup {
  /** The folder's name, which is its stamp, and what a restore asks for. */
  readonly id: string;
  readonly takenAt: Date;
  readonly bytes: number;
  /** Whether the key file travelled with it: without one the copy opens for nobody. */
  readonly openable: boolean;
  /** Without them, a restore leaves the live attachments where they are. */
  readonly attachments: boolean;
}
