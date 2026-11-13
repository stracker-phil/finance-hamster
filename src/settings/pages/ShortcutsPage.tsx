import { useCommands } from "../../queries/commands";
import { Group, Row } from "../controls";

/** Every command and its shortcut, as the menus show them. */
export function ShortcutsPage() {
  const { data = [] } = useCommands();
  return (
    <Group title="Commands" anchor="commands">
      {data.map((command) => (
        <Row key={command.id} label={command.title} description={command.id}>
          {command.shortcut ? <span className="kbd">{command.shortcut}</span> : <span className="settings-value">None</span>}
        </Row>
      ))}
    </Group>
  );
}
