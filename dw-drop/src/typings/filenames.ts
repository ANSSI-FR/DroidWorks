import prettyBytes from "pretty-bytes";
import { locale } from "@/i18n";

export interface FileEntry {
  name: string;
  size: string;
  children: FileEntry[];
}

export enum FileExtension {
  Image,
  Xml,
  Dex,
  Other,
}

function filesTree(tab: { name: string[]; size: string }[]): FileEntry[] {
  const tree = [];
  const partition: { [key: string]: { name: string[]; size: string }[] } = {};
  for (const entry of tab) {
    if (entry.name.length == 0) {
      console.error("empty filename");
    } else if (entry.name.length == 1) {
      tree.push({ name: entry.name[0], size: entry.size, children: [] });
    } else {
      if (entry.name[0] in partition) {
        partition[entry.name[0]].push({
          name: entry.name.slice(1),
          size: entry.size,
        });
      } else {
        partition[entry.name[0]] = [
          {
            name: entry.name.slice(1),
            size: entry.size,
          },
        ];
      }
    }
  }
  for (const pref in partition) {
    tree.push({ name: pref, size: "", children: filesTree(partition[pref]) });
  }
  return tree;
}

export function filenamesTree(
  filenames: { name: string; size: number }[]
): FileEntry[] {
  return filesTree(
    filenames.map((file) => {
      return { name: file.name.split("/"), size: humanReadableSize(file.size) };
    })
  );
}

function humanReadableSize(size: number): string {
  return prettyBytes(size, { locale });
}

export function fileExtension(filename: string): FileExtension {
  let ext = filename.split(".").pop();
  if (ext == undefined) {
    return FileExtension.Other;
  }
  ext = ext.toLowerCase();

  switch (filename.split(".").pop()) {
    case "png":
    case "bmp":
    case "jpg":
      return FileExtension.Image;
    case "xml":
      return FileExtension.Xml;
    case "dex":
      return FileExtension.Dex;
    default:
      return FileExtension.Other;
  }
}
