import os
import gc
from secure_openaiapi import SecureClient, SecureMessage

def main():
    """
    An example demonstrating the secure usage of the SecureClient.
    """
    print("--- Starting secure OpenAI API client example ---")

    # --- 1. Securely Load Credentials ---
    # It's best practice to load secrets from an encrypted store which encrypted with a hardware token
    # rather than hardcoding them in the source code. For the example we can define here
    api_key_bytes = bytearray(b"<EXAMPLE API KEY>")
    base_url_bytes = bytearray(b"https://api.groq.com")

    try:
        # --- 2. Initialize the SecureClient ---
        # The moment the client is created, the content of the bytearrays is
        # copied into memory managed by Rust. This memory is pinned (mlock)
        # to prevent it from being swapped to disk and will be securely
        # zeroed-out when the client object is destroyed.
        print("Initializing SecureClient...")
        client = SecureClient(base_url=bytes(base_url_bytes), api_key=bytes(api_key_bytes))
        print("SecureClient initialized.")

    finally:
        # --- 3. Immediately Clear Python-side Secrets ---
        # Now that the credentials have been copied into secure Rust-managed
        # memory, we should immediately wipe the Python bytearrays.
        # This minimizes the time the secrets are present in Python's memory.
        print("Clearing credentials from Python memory...")
        api_key_bytes[:] = b'\x00' * len(api_key_bytes)
        base_url_bytes[:] = b'\x00' * len(base_url_bytes)

        # We can also delete the variables and suggest garbage collection.
        del api_key_bytes
        del base_url_bytes
        gc.collect()
        print("Python-side credentials cleared.")

    # --- 4. Construct Secure Messages ---
    # All parts of a message, including roles, text, and URLs, must also be
    # passed as bytearrays to ensure they are handled by the secure memory
    # allocator in Rust.
    print("Constructing secure messages...")
    messages = [
        SecureMessage(
            role=b"system",
            content_list=[
                {
                    "type": "text",
                    "text": b"You are a helpful assistant."
                }
            ]
        ),
        SecureMessage(
            role=b"user",
            content_list=[
                {
                    "type": "text",
                    "text": b"What is in this image?"
                },
                {
                    "type": "image_url",
                    "image_url": {
                        # The URL is also a bytearray for secure handling.
                        "url": b"https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/1280px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg"
                    }
                }
            ]
        )
    ]
    print("Secure messages constructed.")

    # --- 5. Make the API Call ---
    # The `chat_completion` method sends the request and returns the API's
    # response in a `SecureBytes` object.
    try:
        print("Sending request to the API...")
        # NOTE: For a real use case, you would specify a real model like "gpt-4o"
        response_content = client.chat_completion(messages=messages, model="meta-llama/llama-4-maverick-17b-128e-instruct")

        # --- 6. Handle the Secure Response ---
        # The `response_content` is a `SecureBytes` object. Its memory is also
        # pinned and will be zeroed on destruction. You can access the data
        # via the `__str__` or `__bytes__` methods.
        print("\n--- Secure API Response ---")
        print(f"Response as string: {str(response_content)}")
        # print(f"Response as bytes: {bytes(response_content)}")
        print("---------------------------\n")

    except Exception as e:
        print(f"\n--- An error occurred ---")
        print(e)
        print("-------------------------\n")


    # --- 7. Automatic Cleanup ---
    # When `client`, `messages`, and `response_content` go out of scope at the
    # end of this function, their Rust `drop` implementations will be called.
    # This automatically zeroes the memory they used (for API key, base URL,
    # all message contents, and the response) and unlocks it.
    print("Example finished. All secure objects will now be dropped and their memory zeroed.")


if __name__ == "__main__":
    main()
