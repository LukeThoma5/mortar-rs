export function makeAction<TApiAction, TActionType extends string>(
  apiAction: TApiAction,
  actionType: TActionType
): TApiAction & {
  toString(): TActionType;
} {
  (apiAction as any).toString = () => actionType;
  return apiAction as TApiAction & {
    toString(): TActionType;
  };
}

type FormDataCommand = "ArrayAppend" | "Append" | "JSON";
type CommandObject = { [key: string]: FormDataCommand };
export function makeFormData<T extends {}>(
  request: T,
  commands: CommandObject
): FormData {
  const formData = new FormData();

  for (const [key, command] of Object.entries(commands)) {
    const value = (request as any)[key];
    if (value === undefined || value === null) {
      continue;
    }
    switch (command) {
      case "ArrayAppend":
        if (Array.isArray(value)) {
          for (const item of value) {
            formData.append(key, item);
          }
        }
        break;
      case "Append":
        formData.append(key, value);
        break;
      case "JSON":
        formData.append(key, JSON.stringify(value));
        break;

      default:
        break;
    }
  }
  return formData;
}
