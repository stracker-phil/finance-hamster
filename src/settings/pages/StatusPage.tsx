import { Group, Row } from "../controls";

/** The checks this page lists come with the data they check (statement chains, PDF checksums, the journal, the
 *  rules files). */
export function StatusPage() {
  return (
    <Group title="Checks" anchor="checks">
      <Row label="Nothing to check yet" description="Statement chains, PDF checksums, the journal and the rules files show here once there is data." />
    </Group>
  );
}
