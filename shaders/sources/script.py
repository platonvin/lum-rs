import os

def list_folders_in_current_directory():
    """
    Prints the names of all subfolders in the current working directory.
    """
    print("Folders in the current directory:")
    try:
        # Get all entries in the current directory
        with os.scandir('.') as entries:
            for entry in entries:
                # Check if the entry is a directory
                if entry.is_dir():
                    print(entry.name)
    except Exception as e:
        print(f"An error occurred: {e}")

if __name__ == "__main__":
    list_folders_in_current_directory()
