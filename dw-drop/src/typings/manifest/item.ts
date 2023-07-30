type Item = {
  isNew?: boolean;
  attributes: { [name: string]: Value | null };
};

type Value = {
  string?: string;
  bool?: boolean;
};

export function itemName(item: Item): string | undefined {
  if (item.attributes.hasOwnProperty("name")) {
    let n = item.attributes["name"];
    if (n == null) {
      return undefined;
    } else {
      return n.string;
    }
  } else {
    return undefined;
  }
}

export function attributeIsTrue(item: Item, attribute: string): boolean | undefined {
  if (item.attributes.hasOwnProperty(attribute)) {
    let a = item.attributes[attribute];
    if (a == null) {
      return undefined;
    }
    if (a.bool == true) {
      return true;
    }
  }
  return false;
}

export default Item;
