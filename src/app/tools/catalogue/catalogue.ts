import { ToolDefinition } from '@core/services/tools/tool.model';

/**
 * Every tool, in its panel's order, each in a folder beside this one. Adding one is an entry
 * here: its name and its line are read from its id, and its component is a chunk of its own,
 * loaded the first time it is opened.
 */
export const TOOLS: readonly ToolDefinition[] = [];
