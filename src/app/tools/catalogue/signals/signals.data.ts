export const SIGNAL_GROUPS = ['terminal', 'stopping', 'faults', 'timers', 'events', 'application'] as const;
export type SignalGroup = (typeof SIGNAL_GROUPS)[number];

export type SignalAction = 'terminate' | 'core' | 'ignore' | 'stop' | 'continue';

export interface UnixSignal {
  readonly name: string;
  /** Linux's, on x86 and ARM; `null` where Linux has none. A range for the real-time ones. */
  readonly linux: string | null;
  /** Only where it differs from Linux's; `null` where macOS has none. */
  readonly macos?: string | null;
  readonly group: SignalGroup;
  readonly action: SignalAction;
  /** SIGKILL and SIGSTOP alone cannot be caught, blocked or ignored. */
  readonly uncatchable?: true;
}

export interface SignalWords {
  readonly signals: Readonly<Record<string, string>>;
}

export const SIGNALS: readonly UnixSignal[] = [
  { name: 'SIGHUP', linux: '1', group: 'terminal', action: 'terminate' },
  { name: 'SIGINT', linux: '2', group: 'terminal', action: 'terminate' },
  { name: 'SIGQUIT', linux: '3', group: 'terminal', action: 'core' },
  { name: 'SIGTSTP', linux: '20', macos: '18', group: 'terminal', action: 'stop' },
  { name: 'SIGTTIN', linux: '21', group: 'terminal', action: 'stop' },
  { name: 'SIGTTOU', linux: '22', group: 'terminal', action: 'stop' },
  { name: 'SIGWINCH', linux: '28', group: 'terminal', action: 'ignore' },
  { name: 'SIGINFO', linux: null, macos: '29', group: 'terminal', action: 'ignore' },

  { name: 'SIGTERM', linux: '15', group: 'stopping', action: 'terminate' },
  { name: 'SIGKILL', linux: '9', group: 'stopping', action: 'terminate', uncatchable: true },
  { name: 'SIGSTOP', linux: '19', macos: '17', group: 'stopping', action: 'stop', uncatchable: true },
  { name: 'SIGCONT', linux: '18', macos: '19', group: 'stopping', action: 'continue' },
  { name: 'SIGABRT', linux: '6', group: 'stopping', action: 'core' },

  { name: 'SIGSEGV', linux: '11', group: 'faults', action: 'core' },
  { name: 'SIGBUS', linux: '7', macos: '10', group: 'faults', action: 'core' },
  { name: 'SIGFPE', linux: '8', group: 'faults', action: 'core' },
  { name: 'SIGILL', linux: '4', group: 'faults', action: 'core' },
  { name: 'SIGTRAP', linux: '5', group: 'faults', action: 'core' },
  { name: 'SIGSYS', linux: '31', macos: '12', group: 'faults', action: 'core' },
  { name: 'SIGSTKFLT', linux: '16', macos: null, group: 'faults', action: 'terminate' },

  { name: 'SIGALRM', linux: '14', group: 'timers', action: 'terminate' },
  { name: 'SIGVTALRM', linux: '26', group: 'timers', action: 'terminate' },
  { name: 'SIGPROF', linux: '27', group: 'timers', action: 'terminate' },
  { name: 'SIGXCPU', linux: '24', group: 'timers', action: 'core' },
  { name: 'SIGXFSZ', linux: '25', group: 'timers', action: 'core' },

  { name: 'SIGCHLD', linux: '17', macos: '20', group: 'events', action: 'ignore' },
  { name: 'SIGPIPE', linux: '13', group: 'events', action: 'terminate' },
  { name: 'SIGURG', linux: '23', macos: '16', group: 'events', action: 'ignore' },
  { name: 'SIGIO', linux: '29', macos: '23', group: 'events', action: 'terminate' },
  { name: 'SIGPWR', linux: '30', macos: null, group: 'events', action: 'terminate' },

  { name: 'SIGUSR1', linux: '10', macos: '30', group: 'application', action: 'terminate' },
  { name: 'SIGUSR2', linux: '12', macos: '31', group: 'application', action: 'terminate' },
  { name: 'SIGRTMIN', linux: '34–64', macos: null, group: 'application', action: 'terminate' },
];
