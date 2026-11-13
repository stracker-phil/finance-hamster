import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const COMMANDS_KEY = ["commands"] as const;

/** Every bus command with its title and shortcut (palette, Shortcuts page). */
export const useCommands = () => useQuery({ queryKey: COMMANDS_KEY, queryFn: api.listCommands });
