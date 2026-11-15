import type { Accent, Appearance, Language, NumberFormat } from "../../api";
import { useT } from "../../i18n";
import { useSettings, useUpdateSettings } from "../../queries/settings";
import { Group, Row, Segmented, Switch } from "../controls";

const ACCENTS: Accent[] = ["system", "orange", "gold", "red", "magenta", "purple", "indigo", "blue", "teal", "graphite"];

export function AppearancePage() {
  const { data: settings } = useSettings();
  const { mutate } = useUpdateSettings();
  const text = useT().settings.appearance;
  return (
    <>
      <Group title={text.languageGroup} anchor="language-group">
        <Row label={text.language} description={text.languageDescription} anchor="language">
          <Segmented<Language>
            label={text.language}
            options={[
              { value: "en", label: "English" },
              { value: "de", label: "Deutsch" },
            ]}
            value={settings?.language}
            onChange={(language) => mutate({ language })}
          />
        </Row>
      </Group>
      <Group title={text.theme} anchor="theme">
        <Row label={text.mode} anchor="mode">
          <Segmented<Appearance>
            label={text.mode}
            options={[
              { value: "system", label: text.automatic },
              { value: "light", label: text.light },
              { value: "dark", label: text.dark },
            ]}
            value={settings?.appearance}
            onChange={(appearance) => mutate({ appearance })}
          />
        </Row>
        <Row label={text.accent} description={text.accentDescription} anchor="accent">
          <div className="accent-dots" role="radiogroup" aria-label={text.accent}>
            {ACCENTS.map((accent) => (
              <button
                key={accent}
                type="button"
                role="radio"
                aria-checked={settings?.accent === accent}
                aria-label={text.accents[accent]}
                title={text.accents[accent]}
                className={`accent-dot accent-dot--${accent}`}
                onClick={() => mutate({ accent })}
              />
            ))}
          </div>
        </Row>
      </Group>
      <Group title={text.amounts} anchor="amounts">
        <Row label={text.numberFormat} anchor="number-format">
          <Segmented<NumberFormat>
            label={text.numberFormat}
            options={[
              { value: "dot_decimal", label: "1,234.56" },
              { value: "comma_decimal", label: "1.234,56" },
            ]}
            value={settings?.numberFormat}
            onChange={(numberFormat) => mutate({ numberFormat })}
          />
        </Row>
        <Switch
          label={text.originalCurrency}
          description={text.originalCurrencyDescription}
          anchor="original-currency"
          checked={settings?.showOriginalCurrency ?? true}
          onChange={(showOriginalCurrency) => mutate({ showOriginalCurrency })}
        />
      </Group>
    </>
  );
}
