import { TextArea, TextField } from "react-aria-components";
import { Controller, useFormContext } from "react-hook-form";
import { FieldHelp } from "@/components/Form/FieldHelp";
import { FieldLabel } from "@/components/Form/FieldLabel";
import formStyles from "@/components/Form/Form.module.css";
import styles from "./config.module.css";
import { jsonObjectProblem } from "./validators";

/** What the template describes, and how the field names it. */
const TEXT = {
  delimited_file: {
    label: "Column template (JSON)",
    help: "The delimiter, header row and columns of the file, written as a JSON object. A destination also needs a file_name.",
  },
  hl7_file: {
    label: "File and header settings (JSON)",
    help: "The file_name of each message ({sample_id}, {control_id}, {message_id}, {date}, {time}, {seq}) and the header fields MSH-3 to MSH-6 (sending_application, sending_facility, receiving_application, receiving_facility), written as a JSON object.",
  },
  pdf_report: {
    label: "Report template (JSON)",
    help: "The labels of the fields to read from the report, written as a JSON object with a fields list; start from the shipped profile.",
  },
};

/**
 * The template of a delimited connector (docs/spec/configuration.md §7.4), a PDF report source
 * (§7.7) or the settings of an HL7 file destination (§7.8): a JSON object in a monospaced box,
 * with a line that says at once whether it can be read.
 */
export function TemplateJsonField({
  name,
  protocol = "delimited_file",
}: {
  name: string;
  protocol?: keyof typeof TEXT;
}) {
  const { control } = useFormContext();
  return (
    <Controller
      control={control}
      name={name}
      rules={{ validate: (value: string) => jsonObjectProblem(value) ?? true }}
      render={({ field, fieldState }) => {
        const problem = jsonObjectProblem(field.value);
        return (
          <TextField
            name={field.name}
            value={field.value}
            onChange={field.onChange}
            onBlur={field.onBlur}
            isRequired
            isInvalid={fieldState.error ? true : undefined}
            validationBehavior="aria"
            className={`${formStyles.field} ${styles.wide}`}
          >
            <FieldLabel isRequired>{TEXT[protocol].label}</FieldLabel>
            <TextArea
              ref={field.ref}
              className={`${formStyles.control} ${styles.template}`}
              spellCheck={false}
              autoCapitalize="off"
              autoCorrect="off"
              rows={10}
            />
            <FieldHelp description={TEXT[protocol].help} errorMessage={fieldState.error?.message} />
            {!fieldState.error && (
              <p role="status" className={problem === null ? styles.valid : styles.note}>
                {problem === null ? "Valid JSON object." : problem}
              </p>
            )}
          </TextField>
        );
      }}
    />
  );
}
