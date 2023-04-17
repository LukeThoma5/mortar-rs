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

// TODO allow them to specify their own formTransform but by default it calls makeFormData.
export function makeFormData<T>(request: T): FormData {
  const formData = new FormData();
  // object keys, for each object if its an array, repeatedly call append.
  // if a file like straight append,
  // if its a complex object then json.stringify
  // otherwise just append it
  return formData;
}
